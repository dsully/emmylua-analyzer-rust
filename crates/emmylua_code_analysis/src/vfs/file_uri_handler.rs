use lsp_types::Uri;
use percent_encoding::percent_decode_str;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;

pub fn file_path_to_uri(path: &PathBuf) -> Option<Uri> {
    Url::from_file_path(path)
        .ok()
        .and_then(|url| Uri::from_str(url.as_str()).ok())
}

pub fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    let url = Url::parse(uri.as_str()).ok()?;
    if url.scheme() != "file" {
        return None;
    }

    let decoded_path = percent_decode_str(url.path())
        .decode_utf8()
        .ok()?
        .to_string();

    let decoded_path = if cfg!(windows)
    {
        let mut windows_decoded_path = decoded_path.trim_start_matches('/').replace('\\', "/");
        if windows_decoded_path.len() >= 2 && windows_decoded_path.chars().nth(1) == Some(':') {
            let drive = windows_decoded_path.chars().next()?.to_ascii_uppercase();
            windows_decoded_path.replace_range(..2, &format!("{}:", drive));
        }

        windows_decoded_path
    } else {
        decoded_path
    };

    Some(PathBuf::from(decoded_path))
}
