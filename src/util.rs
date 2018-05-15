use url::form_urlencoded;

pub fn url_with_query(mut url: String, queries: &[(&str, &str)]) -> String {
    url.push_str("?");
    let len = url.len();
    {
        let mut url_serializer = form_urlencoded::Serializer::for_suffix(&mut url, len);
        for (name, val) in queries {
            url_serializer.append_pair(name, val);
        }
    }
    url
}
