use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use http::{Request, Response};
use tonic::{
    body::BoxBody,
    codegen::{ok, Never, Ready},
    transport::Body,
};
use tower::Service;

use crate::{transaction, utility};

#[derive(Clone)]
pub struct Router {
    pub utility_service: Arc<utility::UtilityService>,
    pub transaction_service: Arc<transaction::TransactionService>,
}

impl Service<()> for Router {
    type Response = Router;
    type Error = Never;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _req: ()) -> Self::Future {
        ok(self.clone())
    }
}

impl Service<Request<Body>> for Router {
    type Response = Response<BoxBody>;
    type Error = Never;
    type Future = Pin<Box<dyn Future<Output = Result<Response<BoxBody>, Never>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut segments = req.uri().path().split('/');
        segments.next();
        let service = segments.next().unwrap();

        match service {
            "utility.Utility" => {
                let me = self.clone();
                Box::pin(async move {
                    let mut svc =
                        utility::model::server::UtilityServer::from_shared(me.utility_service);
                    let mut svc = svc.call(()).await.unwrap();

                    let res = svc.call(req).await.unwrap();
                    Ok(res)
                })
            }

            "transaction.Transaction" => {
                let me = self.clone();
                Box::pin(async move {
                    let mut svc = transaction::model::server::TransactionServer::from_shared(
                        me.transaction_service,
                    );
                    let mut svc = svc.call(()).await.unwrap();

                    let res = svc.call(req).await.unwrap();
                    Ok(res)
                })
            }

            _ => unimplemented!(),
        }
    }
}
