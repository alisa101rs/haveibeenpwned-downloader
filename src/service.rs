use stable_eyre::{eyre::WrapErr, Result};
use tower::{retry::Retry, ServiceBuilder};

use crate::{client::RangeClient, service::retry::Attempts};

pub fn make_service() -> Result<Retry<Attempts, RangeClient>> {
    let client = RangeClient::new().wrap_err("building client")?;
    let client = ServiceBuilder::new().retry(Attempts(10)).service(client);
    Ok(client)
}

mod retry {
    use std::future;

    use tower::retry;

    #[derive(Clone)]
    pub struct Attempts(pub usize);

    impl<T: Clone, Res, E> retry::Policy<T, Res, E> for Attempts {
        type Future = future::Ready<Self>;

        fn retry(&self, _req: &T, result: Result<&Res, &E>) -> Option<Self::Future> {
            match result {
                Ok(_) => {
                    // Treat all `Response`s as success,
                    // so don't retry...
                    None
                }
                Err(_) => {
                    // Treat all errors as failures...
                    // But we limit the number of attempts...
                    if self.0 > 0 {
                        // Try again!
                        Some(future::ready(Attempts(self.0 - 1)))
                    } else {
                        // Used all our attempts, no retry...
                        None
                    }
                }
            }
        }

        fn clone_request(&self, req: &T) -> Option<T> {
            Some(req.clone())
        }
    }
}
