//! The adapter that dresses core's plain channels as havoc-transport's
//! `ClientTransport`. It lives in the binary deliberately: no Cargo edge
//! exists between havoc-core and havoc-transport (§4.2), so the glue is
//! here — which is exactly what stage 4's UDS server will do internally.

use havoc_core::bus::Directed;
use havoc_core::core::{CoreHandle, Session};
use havoc_ipc::Request;
use havoc_transport::{InProcess, Incoming, TransportError};
use tokio::sync::mpsc;

/// Attach a session and merge its three lanes (requests out; directed and
/// broadcast in) into one `InProcess` transport. A lagged broadcast receiver
/// surfaces as `TransportError::Lagged`, never a silent skip.
pub async fn in_process(core: &CoreHandle) -> InProcess {
    let Session {
        id,
        requests,
        mut directed,
        mut broadcast,
    } = core.attach().await;

    let (req_tx, mut req_rx) = mpsc::channel::<Request>(64);
    let (in_tx, in_rx) = mpsc::channel::<Result<Incoming, TransportError>>(256);

    // Requests: tag with our ClientId and forward.
    let requests_out = requests.clone();
    tokio::spawn(async move {
        while let Some(request) = req_rx.recv().await {
            if requests_out.send((id, request)).await.is_err() {
                return;
            }
        }
    });

    // Incoming: merge directed + broadcast into one ordered-enough stream.
    tokio::spawn(async move {
        loop {
            let forwarded = tokio::select! {
                directed_msg = directed.recv() => match directed_msg {
                    Some(Directed::Response(r)) => in_tx.send(Ok(Incoming::Response(r))).await,
                    Some(Directed::Event(e)) => in_tx.send(Ok(Incoming::Event(e))).await,
                    None => {
                        let _ = in_tx.send(Err(TransportError::Closed)).await;
                        return;
                    }
                },
                broadcast_msg = broadcast.recv() => match broadcast_msg {
                    Ok(event) => in_tx.send(Ok(Incoming::Event(event))).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        in_tx.send(Err(TransportError::Lagged(n))).await
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        let _ = in_tx.send(Err(TransportError::Closed)).await;
                        return;
                    }
                },
            };
            if forwarded.is_err() {
                return;
            }
        }
    });

    InProcess {
        requests: req_tx,
        incoming: in_rx,
    }
}
