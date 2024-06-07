//! Serializer and deserializer for the [VCR Cassette
//! format](https://relishapp.com/vcr/vcr/v/6-0-0/docs/cassettes/cassette-format).
//!
//! # Examples
//!
//! Given the following `.json` VCR Cassette recording:
//! ```json
//! {
//!     "http_interactions": [
//!         {
//!             "request": {
//!                 "uri": "http://localhost:7777/foo",
//!                 "body": "",
//!                 "method": "get",
//!                 "headers": { "Accept-Encoding": [ "identity" ] }
//!             },
//!             "response": {
//!                 "body": "Hello foo",
//!                 "http_version": "1.1",
//!                 "status": { "code": 200, "message": "OK" },
//!                 "headers": {
//!                     "Date": [ "Thu, 27 Oct 2011 06:16:31 GMT" ],
//!                     "Content-Type": [ "text/html;charset=utf-8" ],
//!                     "Content-Length": [ "9" ],
//!                 }
//!             },
//!             "recorded_at": "Tue, 01 Nov 2011 04:58:44 GMT"
//!         },
//!     ],
//!     "recorded_with": "VCR 2.0.0"
//! }
//! ```
//!
//! We can deserialize it using [`serde_json`](https://docs.rs/serde-json):
//!
//! ```rust
//! # #![allow(unused)]
//! use std::fs;
//! use vcr_cassette::Cassette;
//!
//! let example = fs::read_to_string("tests/fixtures/example.json").unwrap();
//! let cassette: Cassette = serde_json::from_str(&example).unwrap();
//! ```
//!
//! To deserialize `.yaml` Cassette files use
//! [`serde_yaml`](https://docs.rs/serde-yaml) instead.

#![forbid(unsafe_code, future_incompatible)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, unreachable_pub)]

use std::fmt;
use std::marker::PhantomData;
use std::{collections::HashMap, str::FromStr};

use chrono::{offset::FixedOffset, DateTime};
#[cfg(feature = "regex")]
use regex::Regex;
#[cfg(feature = "regex")]
use serde::de::Unexpected;
use serde::de::{self, Error, MapAccess, Visitor};
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use url::Url;
use void::Void;

pub use chrono;
pub use url;

mod datetime;

/// An HTTP Headers type.
pub type Headers = HashMap<String, Vec<String>>;

/// An identifier of the library which created the recording.
///
/// # Examples
///
/// ```
/// # #![allow(unused)]
/// use vcr_cassette::RecorderId;
///
/// let id: RecorderId = String::from("VCR 2.0.0");
/// ```
pub type RecorderId = String;

/// A sequence of recorded HTTP Request/Response pairs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cassette {
    /// A sequence of recorded HTTP Request/Response pairs.
    pub http_interactions: Vec<HttpInteraction>,

    /// An identifier of the library which created the recording.
    pub recorded_with: RecorderId,
}

/// A single HTTP Request/Response pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpInteraction {
    /// An HTTP response.
    pub response: Response,
    /// An HTTP request.
    pub request: Request,

    /// An [RFC
    /// 2822](https://docs.rs/chrono/0.4.19/chrono/struct.DateTime.html#method.parse_from_rfc2822)
    /// formatted timestamp.
    ///
    /// # Examples
    ///
    /// ```json
    /// { "recorded_at": "Tue, 01 Nov 2011 04:58:44 GMT" }
    /// ```
    #[serde(with = "datetime")]
    pub recorded_at: DateTime<FixedOffset>,
}

/// A recorded HTTP Response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// An HTTP Body.
    pub body: Body,
    /// The version of the HTTP Response.
    pub http_version: Option<Version>,
    /// The Response status
    pub status: Status,
    /// The Response headers
    pub headers: Headers,
}

/// A recorded HTTP Body.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Body {
    /// A bare string, eg `"body": "ohai!"`
    ///
    /// Only matches if the request's body matches the specified string *exactly*.
    String(String),
    /// A string and the request's encoding.  Both must be exactly equal in order for the request
    /// to match this interaction.
    EncodedString {
        /// The manner in which the string was encoded, such as `base64`
        encoding: Option<String>,
        /// The encoded string
        string: String,
    },
    /// A series of [`BodyMatcher`] instances.  All specified matchers must pass in order for the
    /// request to be deemed to match this interaction.
    #[cfg(feature = "matching")]
    Matchers(Vec<BodyMatcher>),

    /// A JSON body.  Mostly useful to make it easier to define a JSON response body without having
    /// to escape a thousand quotes.  Does *not* modify the `Content-Type` response header; you
    /// still have to do that yourself.
    #[cfg(feature = "json")]
    Json(serde_json::Value),
}

impl std::fmt::Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::String(s) => f.write_str(s),
            Self::EncodedString { encoding, string } => if let Some(encoding) = encoding {
                f.write_fmt(format_args!("({encoding}){string}"))
            } else {
                f.write_str(string)
            },
            #[cfg(feature = "matching")]
            Self::Matchers(m) => f.debug_list().entries(m.iter()).finish(),
            #[cfg(feature = "json")]
            Self::Json(j) => f.write_str(&serde_json::to_string(j).expect("invalid JSON body")),
        }
    }
}

impl<'de> Deserialize<'de> for Body {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BodyVisitor(PhantomData<fn() -> Body>);

        impl<'de> Visitor<'de> for BodyVisitor {
            type Value = Body;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Body, E> {
                Ok(FromStr::from_str(value).unwrap())
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Body, M::Error> {
                match map.next_key::<String>()?.as_deref() {
                    Some("encoding") => {
                        let encoding = map.next_value()?;
                        match map.next_key::<String>()?.as_deref() {
                            Some("string") => Ok(Body::EncodedString {
                                encoding,
                                string: map.next_value()?,
                            }),
                            Some(k) => Err(M::Error::unknown_field(k, &["string"])),
                            None => Err(M::Error::missing_field("string")),
                        }
                    }
                    Some("string") => {
                        let string = map.next_value()?;
                        match map.next_key::<String>()?.as_deref() {
                            Some("encoding") => Ok(Body::EncodedString {
                                string,
                                encoding: map.next_value()?,
                            }),
                            Some(k) => Err(M::Error::unknown_field(k, &["encoding"])),
                            None => Err(M::Error::missing_field("encoding")),
                        }
                    }
                    #[cfg(feature = "matching")]
                    Some("matches") => Ok(Body::Matchers(map.next_value()?)),
                    #[cfg(feature = "json")]
                    Some("json") => Ok(Body::Json(map.next_value()?)),
                    Some(k) => Err(M::Error::unknown_field(
                        k,
                        &[
                            "encoding",
                            "string",
                            #[cfg(feature = "matching")]
                            "matches",
                            #[cfg(feature = "json")]
                            "json",
                        ],
                    )),
                    None => {
                        // OK this is starting to get silly
                        #[cfg(all(feature = "matching", feature = "json"))]
                        let fields = "matches, json, encoding, or string";
                        #[cfg(all(feature = "matching", not(feature = "json")))]
                        let fields = "matches, encoding, or string";
                        #[cfg(all(not(feature = "matching"), feature = "json"))]
                        let fields = "json, encoding, or string";
                        // Yes, DeMorgan says there's a better way to do this, but it's visually
                        // more similar to the previous versions, so it's more readable, IMO
                        #[cfg(all(not(feature = "matching"), not(feature = "json")))]
                        let fields = "encoding or string";

                        Err(M::Error::missing_field(fields))
                    }
                }
            }
        }

        deserializer.deserialize_any(BodyVisitor(PhantomData))
    }
}

impl Serialize for Body {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::String(s) => ser.serialize_str(s),
            Self::EncodedString { encoding, string } => {
                let mut map = ser.serialize_map(Some(2))?;
                map.serialize_entry("string", string)?;
                map.serialize_entry("encoding", encoding)?;
                map.end()
            }
            #[cfg(feature = "matching")]
            Self::Matchers(m) => {
                let mut map = ser.serialize_map(Some(1))?;
                map.serialize_entry("matches", m)?;
                map.end()
            }
            #[cfg(feature = "json")]
            Self::Json(j) => {
                let mut map = ser.serialize_map(Some(1))?;
                map.serialize_entry("json", j)?;
                map.end()
            }
        }
    }
}

impl PartialEq for Body {
    fn eq(&self, other: &Body) -> bool {
        match self {
            Self::String(s) => match other {
                Self::String(o) => s == o,
                Self::EncodedString { encoding, string } => encoding.is_none() && s == string,
                #[cfg(feature = "matching")]
                Self::Matchers(_) => other.eq(self),
                #[cfg(feature = "json")]
                Self::Json(j) => serde_json::to_string(j).expect("invalid JSON body") == *s,
            },
            Self::EncodedString { encoding, string } => match other {
                Self::String(s) => encoding.is_none() && s == string,
                Self::EncodedString {
                    encoding: oe,
                    string: os,
                } => encoding == oe && string == os,
                #[cfg(feature = "matching")]
                Self::Matchers(_) => false,
                #[cfg(feature = "json")]
                Self::Json(_) => false,
            },
            #[cfg(feature = "matching")]
            Self::Matchers(matchers) => match other {
                Self::String(s) => matchers.iter().all(|m| m.matches(s)),
                Self::EncodedString { .. } => false,
                #[cfg(feature = "matching")]
                Self::Matchers(_) => false,
                #[cfg(feature = "json")]
                Self::Json(j) => {
                    let s = serde_json::to_string(j).expect("invalid JSON body");
                    matchers.iter().all(|m| m.matches(&s))
                }
            },
            #[cfg(feature = "json")]
            Self::Json(_) => other.eq(self),
        }
    }
}

/// A mechanism for determining if a request body matches a specified substring or regular
/// expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BodyMatcher {
    /// The body must contain exactly the string specified.
    #[serde(rename = "substring")]
    Substring(String),
    /// The body must match the specified regular expression.
    #[cfg(feature = "regex")]
    #[serde(
        rename = "regex",
        deserialize_with = "parse_regex",
        serialize_with = "serialize_regex"
    )]
    Regex(Regex),
}

#[cfg(feature = "regex")]
fn parse_regex<'de, D: Deserializer<'de>>(d: D) -> Result<Regex, D::Error> {
    struct RegexVisitor(PhantomData<fn() -> Regex>);

    impl<'de> Visitor<'de> for RegexVisitor {
        type Value = Regex;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("valid regular expression as a string")
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<Regex, E> {
            Regex::new(s).map_err(|_| {
                E::invalid_value(Unexpected::Other("invalid regular expression"), &self)
            })
        }
    }

    d.deserialize_str(RegexVisitor(PhantomData))
}

#[cfg(feature = "regex")]
fn serialize_regex<S: Serializer>(r: &Regex, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(r.as_str())
}

#[cfg(feature = "matching")]
impl BodyMatcher {
    fn matches(&self, s: &str) -> bool {
        match self {
            Self::Substring(m) => s.contains(m),
            #[cfg(feature = "regex")]
            Self::Regex(r) => r.is_match(s),
        }
    }
}

impl FromStr for Body {
    // This implementation of `from_str` can never fail, so use the impossible
    // `Void` type as the error type.
    type Err = Void;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Body::String(s.to_string()))
    }
}

/// A recorded HTTP Status Code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    /// The HTTP status code.
    pub code: u16,
    /// The HTTP status message.
    pub message: String,
}

/// A recorded HTTP Request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// The Request URI.
    pub uri: Url,
    /// The Request body.
    pub body: Body,
    /// The Request method.
    pub method: Method,
    /// The Request headers.
    pub headers: Headers,
}

/// An HTTP method.
///
/// WebDAV and custom methods can be created by passing a static string to the
/// `Other` member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    /// An HTTP `CONNECT` method.
    Connect,
    /// An HTTP `DELETE` method.
    Delete,
    /// An HTTP `GET` method.
    Get,
    /// An HTTP `HEAD` method.
    Head,
    /// An HTTP `OPTIONS` method.
    Options,
    /// An HTTP `PATCH` method.
    Patch,
    /// An HTTP `POST` method.
    Post,
    /// An HTTP `PUT` method.
    Put,
    /// An HTTP `TRACE` method.
    Trace,
    /// Any other HTTP method.
    Other(String),
}

impl Method {
    /// Convert the HTTP method to its string representation.
    pub fn as_str(&self) -> &str {
        match self {
            Method::Connect => "CONNECT",
            Method::Delete => "DELETE",
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Patch => "PATCH",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Trace => "TRACE",
            Method::Other(s) => &s,
        }
    }
}

/// The version of the HTTP protocol in use.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[non_exhaustive]
pub enum Version {
    /// HTTP/0.9
    #[serde(rename = "0.9")]
    Http0_9,

    /// HTTP/1.0
    #[serde(rename = "1.0")]
    Http1_0,

    /// HTTP/1.1
    #[serde(rename = "1.1")]
    Http1_1,

    /// HTTP/2.0
    #[serde(rename = "2")]
    Http2_0,

    /// HTTP/3.0
    #[serde(rename = "3")]
    Http3_0,
}
