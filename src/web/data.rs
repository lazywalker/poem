use std::ops::Deref;

use crate::{error::GetDataError, FromRequest, Request, RequestBody, Result};

/// An extractor that can extract data from the request extension.
///
/// # Example
///
/// ```
/// use poem::{handler, middleware::AddData, route, route::get, web::Data, EndpointExt};
///
/// #[handler]
/// async fn index(data: Data<&i32>) {
///     assert_eq!(*data.0, 10);
/// }
///
/// let mut app = route().at("/", get(index)).with(AddData::new(10));
/// ```
pub struct Data<T>(pub T);

impl<T> Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: Send + Sync + 'static> FromRequest<'a> for Data<&'a T> {
    type Error = GetDataError;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        req.extensions()
            .get::<T>()
            .ok_or_else(|| GetDataError(std::any::type_name::<T>()))
            .map(Data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, http::StatusCode, middleware::AddData, Endpoint, EndpointExt};

    #[tokio::test]
    async fn test_data_extractor() {
        #[handler(internal)]
        async fn index(value: Data<&i32>) {
            assert_eq!(value.0, &100);
        }

        let app = index.with(AddData::new(100i32));
        app.call(Request::default()).await;
    }

    #[tokio::test]
    async fn test_data_extractor_error() {
        #[handler(internal)]
        async fn index(_value: Data<&i32>) {
            todo!()
        }

        let app = index;
        let mut resp = app.call(Request::default()).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            resp.take_body().into_string().await.unwrap(),
            "data of type `i32` was not found."
        );
    }

    #[tokio::test]
    async fn test_data_extractor_deref() {
        #[handler(internal)]
        async fn index(value: Data<&String>) {
            assert_eq!(value.to_uppercase(), "ABC");
        }
        let app = index.with(AddData::new("abc".to_string()));
        app.call(Request::default()).await;
    }
}
