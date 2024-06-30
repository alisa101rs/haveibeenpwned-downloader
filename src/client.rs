use std::{task::Poll, time::Duration};

use bytes::Bytes;
use reqwest::{ClientBuilder, Method, Request, Url};
use stable_eyre::{eyre::Context, Result};
use tower::Service;
use tracing::instrument;

pub type HashPrefix = [u8; 5];

#[derive(Clone)]
pub struct RangeClient {
    client: reqwest::Client,
}

impl RangeClient {
    const BASE_URI: &'static str = "https://api.pwnedpasswords.com/range/";

    pub fn new() -> Result<Self> {
        let client = ClientBuilder::new()
            .http2_keep_alive_interval(Some(Duration::from_millis(10000)))
            .http2_prior_knowledge()
            .http2_adaptive_window(true)
            .timeout(Duration::from_secs(5))
            .build()
            .wrap_err("Failed to build http client")?;

        Ok(Self { client })
    }

    #[instrument(skip(self), err)]
    pub async fn get(&self, range: HashPrefix) -> Result<(HashPrefix, Bytes)> {
        self.clone().call(range).await
    }
}

impl Service<HashPrefix> for RangeClient {
    type Response = (HashPrefix, Bytes);
    type Error = stable_eyre::Report;

    type Future = future::RangeFuture;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HashPrefix) -> Self::Future {
        let request = Request::new(
            Method::GET,
            Url::parse(RangeClient::BASE_URI)
                .expect("valid url")
                .join(std::str::from_utf8(req.as_ref()).expect("range to be valid utf8"))
                .expect("valid url"),
        );

        let reqwest = self.client.call(request);

        future::RangeFuture::Reqwest { reqwest, req }
    }
}

mod future {
    use std::{
        future::Future,
        pin::Pin,
        task::{ready, Context, Poll},
    };

    use bytes::Bytes;
    use http_body_util::BodyExt;
    use pin_project_lite::pin_project;
    use stable_eyre::{eyre::WrapErr, Result};

    use crate::client::HashPrefix;

    type ReqwestFuture = <reqwest::Client as tower::Service<reqwest::Request>>::Future;
    type BodyFuture = http_body_util::combinators::Collect<reqwest::Body>;

    pin_project! {
        #[project = EnumProj]
        pub enum RangeFuture {
            Reqwest {
                #[pin] reqwest: ReqwestFuture,
                req: HashPrefix,
            },
            Body {
                #[pin] body: BodyFuture,
                req: HashPrefix,
            }
        }
    }

    impl Future for RangeFuture {
        type Output = Result<(HashPrefix, Bytes)>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.as_mut().project() {
                EnumProj::Reqwest {
                    reqwest,
                    req: &mut req,
                } => {
                    let response = match ready!(reqwest.poll(cx)).wrap_err("Failed to get response")
                    {
                        Ok(r) => r,
                        Err(er) => return Poll::Ready(Err(er)),
                    };
                    let body: reqwest::Body = response.into();
                    let body = body.collect();
                    Pin::set(&mut self, RangeFuture::Body { body, req });

                    Self::poll(self, cx)
                }
                EnumProj::Body { body, req } => {
                    match ready!(body.poll(cx)).wrap_err("Failed to get response body") {
                        Ok(b) => Poll::Ready(Ok((*req, b.to_bytes()))),
                        Err(er) => Poll::Ready(Err(er)),
                    }
                }
            }
        }
    }
}
