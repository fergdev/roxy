use url::Url;

pub fn set_url_authority(url: &mut Url, auth: &str) -> Result<(), String> {
    if auth.contains('@') {
        let mut split = auth.split('@');
        let user = split.next().ok_or("Missing username".to_string())?;
        let host = split.next().ok_or("Missing password".to_string())?;

        let mut user = user.split(':');
        let username = user.next().unwrap_or("");
        let password = user.next().unwrap_or("");

        let mut host = host.split(':');
        let hostname = host.next().unwrap_or("");
        let port = host.next().unwrap_or("");

        let _ = url::quirks::set_username(url, username);
        let _ = url::quirks::set_password(url, password);
        let _ = url::quirks::set_host(url, hostname);
        let _ = url::quirks::set_port(url, port);
    } else {
        let mut host = auth.split(':');
        let hostname = host.next().unwrap_or("");
        let port = host.next().unwrap_or("");

        let _ = url::quirks::set_host(url, hostname);
        let _ = url::quirks::set_port(url, port);
    }
    Ok(())
}
