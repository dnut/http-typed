use std::{any::type_name, marker::PhantomData};

use reqwest::header::CONTENT_TYPE;

use crate::{All, HttpMethod, InRequestGroup, Request, SerializeBody};

/// A client to delegate to the send function that provides the ability to
/// optionally specify:
/// - a base url to be used for all requests
/// - a request group to constrain the request types accepted by this type
pub struct Client<RequestGroup = All> {
    base_url: String,
    inner: reqwest::Client,
    _p: PhantomData<RequestGroup>,
}

/// Explicitly implemented to avoid requirement RequestGroup: Debug
impl<RequestGroup> std::fmt::Debug for Client<RequestGroup> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("base_url", &self.base_url)
            .field("inner", &self.inner)
            .finish()
    }
}

/// Explicitly implemented to avoid requirement RequestGroup: Default
impl<RequestGroup> Default for Client<RequestGroup> {
    fn default() -> Self {
        Self {
            base_url: Default::default(),
            inner: Default::default(),
            _p: PhantomData,
        }
    }
}

/// Explicitly implemented to avoid requirement RequestGroup: Clone
impl<RequestGroup> Clone for Client<RequestGroup> {
    fn clone(&self) -> Self {
        Self {
            base_url: self.base_url.clone(),
            inner: self.inner.clone(),
            _p: PhantomData,
        }
    }
}

impl<RequestGroup> Client<RequestGroup> {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            inner: reqwest::Client::new(),
            _p: PhantomData,
        }
    }

    /// Send the provided request to the host at this client's base_url, using
    /// the Request implementation to determine the remaining url path and
    /// request data.
    ///
    /// The url used for the request is {self.base_url}{request.path()}
    pub async fn send<Req>(
        &self,
        request: Req,
    ) -> Result<Req::Response, Error<<Req::Serializer as SerializeBody<Req>>::Error>>
    where
        Req: Request + InRequestGroup<RequestGroup>,
        Req::Response: for<'a> serde::Deserialize<'a>,
    {
        send_custom_with_client(
            &self.inner,
            &format!("{}{}", self.base_url, request.path()),
            request.method(),
            request,
        )
        .await
    }

    /// Send the provided request to the host at this client's base_url plus
    /// url_infix, using the Request implementation to determine the remaining
    /// url path and request data.
    ///
    /// The url used for the request is
    /// {self.base_url}{url_infix}{request.path()}
    ///
    /// If you'd like to specify the entire base url for each request using this
    /// method, instantiate this struct with base_url = "" (the default)
    pub async fn send_to<Req>(
        &self,
        url_infix: &str,
        request: Req,
    ) -> Result<Req::Response, Error<<Req::Serializer as SerializeBody<Req>>::Error>>
    where
        Req: Request + InRequestGroup<RequestGroup>,
        Req::Response: for<'a> serde::Deserialize<'a>,
    {
        send_custom_with_client(
            &self.inner,
            &format!("{}{url_infix}{}", self.base_url, request.path()),
            request.method(),
            request,
        )
        .await
    }

    /// Send the provided request to the specified path using the specified method,
    /// and deserialize the response into the specified response type.
    ///
    /// The url used for this request is {self.base_url}{path}
    ///
    /// If you'd like to specify the entire base url for each request using this
    /// method, instantiate this struct with base_url = "" (the default)
    pub async fn send_custom<Req, Res>(
        &self,
        path: &str,
        method: HttpMethod,
        request: Req,
    ) -> Result<Res, Error<Req::Error>>
    where
        Req: SimpleBody,
        Res: for<'a> serde::Deserialize<'a>,
    {
        send_custom_with_client(
            &self.inner,
            &format!("{}{path}", self.base_url),
            method,
            request,
        )
        .await
    }
}

/// Convenience function to create a client and send a request using minimal
/// boilerplate. Creating a client is expensive, so you should not use this
/// function if you plan on sending multiple requests.
///
/// Equivalent to:
/// - `Client::new(base_url).send(request)`
/// - `Client::default().send_to(base_url, request)`
///
/// Send the provided request to the host at the specified base url, using the
/// request metadata specified by the Request implementation to create the http
/// request and determine the response type.
///
/// The url used for the request is {base_url}{request.path()}
pub async fn send<Req>(
    base_url: &str,
    request: Req,
) -> Result<Req::Response, Error<<Req::Serializer as SerializeBody<Req>>::Error>>
where
    Req: Request,
    Req::Response: for<'a> serde::Deserialize<'a>,
{
    let url = format!("{base_url}{}", request.path());
    send_custom_with_client(&reqwest::Client::new(), &url, request.method(), request).await
}

/// Convenience function to create a client and send a request using minimal
/// boilerplate. Creating a client is expensive, so you should not use this
/// function if you plan on sending multiple requests.
///
/// Equivalent to:
/// - `Client::default().send_custom(url, method, request)`
/// - `Client::new(url).send_custom("", method, request)`
///
/// Send the provided request to the specified url using the specified method,
/// and deserialize the response into the specified response type.
pub async fn send_custom<Req, Res>(
    url: &str,
    method: HttpMethod,
    request: Req,
) -> Result<Res, Error<Req::Error>>
where
    Req: SimpleBody,
    Res: for<'a> serde::Deserialize<'a>,
{
    send_custom_with_client(&reqwest::Client::new(), url, method, request).await
}

async fn send_custom_with_client<Req, Res>(
    client: &reqwest::Client,
    url: &str,
    method: HttpMethod,
    request: Req,
) -> Result<Res, Error<Req::Error>>
where
    Req: SimpleBody,
    Res: for<'a> serde::Deserialize<'a>,
{
    let response = client
        .request(method.into(), url)
        .body(request.simple_body().map_err(Error::SerializationError)?)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        let body = response.bytes().await?;
        serde_json::from_slice(&body).map_err(|error| Error::DeserializationError {
            error,
            response_body: body_bytes_to_str(&body),
        })
    } else {
        let message = match response.bytes().await {
            Ok(bytes) => body_bytes_to_str(&bytes),
            Err(e) => format!("failed to get body: {e:?}"),
        };
        Err(Error::InvalidStatusCode(status.into(), message))
    }
}

/// This allows the send_custom methods to accept objects that do not implement
/// Request. SimpleBody is a more minimal requirement that you get automatically
/// if you implement request, but you can also implement this by itself without
/// implementing Request, to keep things simple with usages of send_custom.
pub trait SimpleBody {
    type Error;
    fn simple_body(&self) -> Result<Vec<u8>, Self::Error>;
}

impl<T: Request> SimpleBody for T {
    type Error = <<Self as Request>::Serializer as SerializeBody<Self>>::Error;

    fn simple_body(&self) -> Result<Vec<u8>, Self::Error> {
        <Self as Request>::Serializer::serialize_body(self)
    }
}

fn body_bytes_to_str(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(message) => message.to_owned(),
        Err(e) => format!("could not read message body as a string: {e:?}"),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error<Ser = serde_json::error::Error> {
    #[error("reqwest error: {0}")]
    ClientError(#[from] reqwest::Error),
    #[error("request body serialization error: {0}")]
    SerializationError(Ser),
    #[error("serde deserialization error `{error}` while parsing response body: {response_body}")]
    DeserializationError {
        error: serde_json::error::Error,
        response_body: String,
    },
    #[error("invalid status code {0} with response body: `{1}`")]
    InvalidStatusCode(u16, String),
}

impl From<HttpMethod> for reqwest::Method {
    fn from(value: HttpMethod) -> Self {
        match value {
            HttpMethod::Options => reqwest::Method::OPTIONS,
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Trace => reqwest::Method::TRACE,
            HttpMethod::Connect => reqwest::Method::CONNECT,
            HttpMethod::Patch => reqwest::Method::PATCH,
        }
    }
}
