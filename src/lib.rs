//! HTTP client supporting custom request and response types. Pass any type into
//! a `send` function or method, and it will return a result of your desired
//! response type. `send` handles request serialization, http messaging, and
//! response deserialization.
//!
//! To keep this crate simple, it is is oriented towards a specific but very
//! common pattern. If your use case meets the following conditions, this crate
//! will work for you:
//! 1. request-response communication
//! 2. async rust functions
//! 3. communicate over http (uses reqwest under the hood)
//! 4. http body is serialized as json
//! 5. status codes outside the 200 range are considered errors
//! 6. request and response types must be serializable and deserializable using
//!    serde
//! 7. the path and HTTP method can be determined from the concrete rust type
//!    used for the request
//!
//! ## Usage
//!
//! ### Typical
//!
//! To use this library, your request and response types must implement
//! serde::Serialize and serde::Deserialize, respectively.
//!
//! To take full advantage of all library features, you can implement `Request`
//! for each of your request types, instantiate a `Client`, and then you can
//! simply invoke `Client::send` to send requests.
//!
//! ```rust
//! let client = Client::new("http://example.com");
//! let response = client.send(MyRequest::new()).await?;
//! ```
//!
//! ### Basic
//!
//! If you don't want to implement Request or create a Client, the most manual
//! and basic way to use this library is by using `send_custom`.
//!
//! ```rust
//! let my_response: MyResponse = send_custom(
//!     "http://example.com/path/to/my/request/",
//!     HttpMethod::Get,
//!     MyRequest::new()
//! )
//! .await?;
//! ```
//!
//! ### Client
//!
//! One downside of the `send_custom` (and `send`) *function* is that it
//! instantiates a client for every request, which is expensive. To improve
//! performance, you can use the `Client::send_custom` (and `Client::send`)
//! *method* instead to re-use an existing client for every request.
//!
//! ```rust
//! let client = Client::default();
//! let my_response: MyResponse = client.send_custom(
//!     "http://example.com/path/to/my/request/",
//!     HttpMethod::Get,
//!     MyRequest::new()
//! )
//! .await?;
//! ```
//!
//! ### Request
//!
//! You may also prefer not to specify metadata about the request every time you
//! send a request, since these things will likely be the same for every request
//! of this type. Describe the request metadata in the type system by
//! implementing the Request trait.
//!
//! ```rust
//! pub trait Request {
//!     type Response;
//!     fn method(&self) -> HttpMethod;
//!     fn path(&self) -> String;
//! }
//! ```
//!
//! This increases design flexibility and reduces boilerplate. See the [API
//! Client Design](#api-client-design) section below for an explanation.
//!
//! If you do not control the crate with the request and response structs, you
//! can implement any traits for them using the newtype pattern, or with a
//! reusable generic wrapper struct.
//!
//! After implementing this trait, you can use the send function and method,
//! which requires the base url to be included, instead of the full url. All
//! other information about how to send the request and response is implied by
//! the type of the input. This still creates a client on every request, so the
//! performance is not optimal if you are sending multiple requests.
//!
//! ```rust
//! let my_response = send("http://example.com", MyRequest::new()).await?;
//! // The type of my_response is determined by the trait's associated type.
//! // It does not need to be inferrable from the calling context.
//! return my_response.some_field
//! ```
//!
//! If you want to send multiple requests, or if you don't want to include the
//! base url when calling `send`, instantiate a Client:
//!
//! ```rust
//! let client = Client::new("http://example.com");
//! let my_response = client.send(MyRequest::new()).await?;
//! ```
//!
//! ### Request groups
//!
//! You can also define request groups. This defines a client type that is
//! explicit about exactly which requests it can handle. The code will not
//! compile if you try to send a request with the wrong client.
//!
//! ```rust
//! request_group!(MyApi { MyRequest1, MyRequest2 });
//! ```
//! ```rust
//! let my_client = Client::<MyApi>::new("http://example.com");
//! let my_response1 = my_client.send(MyRequest1::new()).await?; // works
//! let other_response = my_client.send(OtherRequest::new()).await?; // does not compile
//! ```
//!
//! ### send_to
//!
//! If you want to restrict the request group, but still want to include the url
//! for every call to `send`, `MyClient` has a `send_to` method that can be used
//! with the default client to specify the url at the call-site.
//! ```rust
//! let my_client = Client::<MyApi>::default();
//! let my_response2 = my_client.send_to("http://example.com", MyRequest2::new()).await?; // works
//! let other_response = my_client.send_to("http://example.com", OtherRequest::new()).await?; // does not compile
//! ```
//!
//! The send_to method can also be used to insert a string after the base_url
//! and before the Request path.
//!
//! ```rust
//! let my_client = Client::new("http://example.com");
//! let my_response = my_client.send_to("/api/v2", MyRequest::new()).await?;
//! ```
//!
//! ## API Client Design
//! Normally, a you might implement a custom client struct to connect to an API,
//! including a custom method for every request. In doing so, you've forced all
//! dependents of the API to make a choice between two options:
//! 1. use the specific custom client struct that was already implemented,
//!    accepting any issues with it.
//! 2. implement a custom client from scratch, re-writing and maintaining all
//!    the details about each request, including what http method to use, what
//!    path to use, how to serialize/deserialized the message, etc.
//!
//! Instead, you can describe the metadata through trait definitions for
//! ultimate flexibility, without locking dependents into a client
//! implementation or needing to implement any custom clients structs.
//! Dependents of the API now have better options:
//! 1. use the Client struct provided by http-typed, accepting any issues with
//!    it. This is easier for you to support because you don't need to worry
//!    about implementation details of sending requests in general. You can just
//!    export or alias the `Client` struct.
//! 2. implement a custom client that can generically handle types implementing
//!    Request by using the data returned by their methods. This is easier for
//!    dependents because they don't need to write any request-specific code.
//!    The Request trait exposes that information without locking them into a
//!    client implementation. Only a single generic request handler is
//!    sufficient.
//!
//!
//! ## Other use cases
//! If your use case does not meet some of the conditions 2-7 described in the
//! introduction, you'll find my other crate useful, which individually
//! generalizes each of those, allowing any of them to be individually
//! customized with minimal boilerplate. It is currently a work in progress, but
//! almost complete. This crate and that crate will be source-code-compatible,
//! meaning the other crate can be used as a drop-in replacement of this one
//! without changing any code, just with more customization available.

use std::{any::type_name, marker::PhantomData};

use reqwest::header::CONTENT_TYPE;

pub trait Request {
    /// Type to deserialize from the http response body
    type Response;
    /// HTTP method that the request will be sent with
    fn method(&self) -> HttpMethod;
    /// String to appended to the end of url when sending this request.
    fn path(&self) -> String;
}

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
    pub async fn send<Req>(&self, request: Req) -> Result<Req::Response, Error>
    where
        Req: Request + serde::Serialize + InRequestGroup<RequestGroup>,
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
    pub async fn send_to<Req>(&self, url_infix: &str, request: Req) -> Result<Req::Response, Error>
    where
        Req: Request + serde::Serialize + InRequestGroup<RequestGroup>,
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
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize,
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
pub async fn send<Req>(base_url: &str, request: Req) -> Result<Req::Response, Error>
where
    Req: Request + serde::Serialize,
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
) -> Result<Res, Error>
where
    Req: serde::Serialize,
    Res: for<'a> serde::Deserialize<'a>,
{
    send_custom_with_client(&reqwest::Client::new(), url, method, request).await
}

async fn send_custom_with_client<Req, Res>(
    client: &reqwest::Client,
    url: &str,
    method: HttpMethod,
    request: Req,
) -> Result<Res, Error>
where
    Req: serde::Serialize,
    Res: for<'a> serde::Deserialize<'a>,
{
    let response = client
        .request(method.into(), url)
        .body(
            serde_json::to_string(&request)
                .map_err(Error::SerializationError)?
                .into_bytes(),
        )
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

fn body_bytes_to_str(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(message) => message.to_owned(),
        Err(e) => format!("could not read message body as a string: {e:?}"),
    }
}

/// Define a request group to constrain which requests can be used with a client.
/// ```ignore
/// request_group!(MyApi { MyRequest1, MyRequest2 });
/// ```
#[macro_export]
macro_rules! request_group {
    ($viz:vis $Name:ident { $($Request:ident),*$(,)? }) => {
        $viz struct $Name;
        $(impl $crate::InRequestGroup<$Name> for $Request {})*
    };
}

/// Indicates that a request is part of a request group. If you use the
/// request_group macro to define the group, it will handle the implementation
/// of this trait automatically.
pub trait InRequestGroup<Group> {}

/// The default group. All requests are in this group.
pub struct All;
impl<T> InRequestGroup<All> for T {}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("reqwest error: {0}")]
    ClientError(#[from] reqwest::Error),
    #[error("serde serialization error: {0}")]
    SerializationError(serde_json::error::Error),
    #[error("serde deserialization error `{error}` while parsing response body: {response_body}")]
    DeserializationError {
        error: serde_json::error::Error,
        response_body: String,
    },
    #[error("invalid status code {0} with response body: `{1}`")]
    InvalidStatusCode(u16, String),
}

#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
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
