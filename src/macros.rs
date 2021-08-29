use rocket::{http::Status, response::status};
use std::fmt::Debug;

macro_rules! call {
    ($receiver:ident.$msg:expr) => {
        match <$receiver as xactor::Service>::from_registry().await {
            Ok(addr) => addr
                .call($msg)
                .await
                .map_err(|err| anyhow::anyhow!(err))
                .flatten(),
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    };
}

pub(crate) use call;

macro_rules! send {
    ($receiver:ident.$msg:expr) => {
        match <$receiver as xactor::Service>::from_registry().await {
            Ok(addr) => addr.send($msg).map_err(|err| anyhow::anyhow!(err)),
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    };
}

pub(crate) use send;

pub trait ToRocket {
    type O;
    fn to_rocket<S: ToString>(
        self,
        message: S,
        status: Status,
    ) -> Result<Self::O, status::Custom<String>>;
}

impl<O, E> ToRocket for Result<O, E>
where
    E: Debug,
{
    type O = O;
    fn to_rocket<S: ToString>(
        self,
        message: S,
        status: Status,
    ) -> Result<Self::O, status::Custom<String>> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => {
                error!("{}", format!("{}: {:?}", message.to_string(), err));
                Err(status::Custom(status, message.to_string()))
            }
        }
    }
}
