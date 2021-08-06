macro_rules! try_fut {
    ($t:expr) => {
        match $t {
            Err(err) => return Box::pin(async move { Err(err) }),
            Ok(ok) => ok,
        }
    };
}
pub(crate) use try_fut;

macro_rules! bail_fut {
    ($($t:tt)*) => {
        return Box::pin(async move { Err(anyhow!($($t)*)) });
    }
}
pub(crate) use bail_fut;
