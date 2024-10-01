use actix_web::{
    body::EitherBody,
    http::header::{Accept, Header},
    http::StatusCode,
    web::Json,
    HttpResponseBuilder, Responder,
};
use serde::Serialize;

/// Responder for serializing a list using a client-specified format, e.g. csv or json.
pub struct Ser<T: Serialize>(pub Vec<T>);

impl<T: Serialize> Responder for Ser<T> {
    type Body = EitherBody<String>;

    fn respond_to(self, request: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let mimes = Accept::parse(request)
            .ok()
            .map(|accept| accept.ranked())
            .unwrap_or_default();

        for mime in mimes {
            match mime.essence_str() {
                "application/json" => break, // JSON is the default
                "text/csv" => {
                    let mut s = Vec::<u8>::new();
                    let mut w = csv::WriterBuilder::new()
                        .has_headers(true)
                        .terminator(csv::Terminator::Any(b'\n'))
                        .from_writer(&mut s);

                    for element in &self.0 {
                        w.serialize(element)
                            .expect("Failed to serialize value as csv");
                    }
                    drop(w);

                    let s = String::from_utf8(s).unwrap();
                    let body = EitherBody::new(s);
                    return HttpResponseBuilder::new(StatusCode::OK)
                        .insert_header(("Content-Type", "text/csv"))
                        .message_body(body)
                        .unwrap();
                }
                _ => continue,
            };
        }

        Json(self.0).respond_to(request)
    }
}
