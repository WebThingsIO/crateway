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
