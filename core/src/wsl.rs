//! A checkout inside a WSL distro, as Windows names it
//! (`\\wsl.localhost\Debian\home\me\app`) and as Linux does (`/home/me/app`).
//! On Windows, Builder runs the app's Makefile and the agent inside the
//! distro, so commands get the Linux form and file checks the Windows one.

const PREFIXES: [&str; 4] = [r"\\?\UNC\wsl.localhost\", r"\\?\UNC\wsl$\", r"\\wsl.localhost\", r"\\wsl$\"];

/// `(distro, linux path)` for a path under `\\wsl.localhost` or `\\wsl$`.
pub fn location(path: &str) -> Option<(String, String)> {
    let path = path.trim().replace('/', "\\");
    let rest = PREFIXES.iter().find_map(|prefix| {
        path.get(..prefix.len())
            .filter(|head| head.eq_ignore_ascii_case(prefix))
            .map(|_| &path[prefix.len()..])
    })?;
    let mut parts = rest.split('\\').filter(|part| !part.is_empty());
    let distro = parts.next()?.to_string();
    let linux = format!("/{}", parts.collect::<Vec<_>>().join("/"));
    Some((distro, linux))
}

/// The Windows name of `linux` inside `distro`.
pub fn unc(distro: &str, linux: &str) -> String {
    format!(r"\\wsl.localhost\{}{}", distro, linux.replace('/', "\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_in_every_spelling() {
        let want = Some(("Debian".to_string(), "/home/nir/voipappz/app".to_string()));
        assert_eq!(location(r"\\wsl.localhost\Debian\home\nir\voipappz\app"), want);
        assert_eq!(location(r"\\wsl$\Debian\home\nir\voipappz\app\"), want);
        assert_eq!(location(r"\\?\UNC\wsl.localhost\Debian\home\nir\voipappz\app"), want);
        assert_eq!(location("//wsl.localhost/Debian/home/nir/voipappz/app"), want);
        assert_eq!(location(r"\\WSL.LOCALHOST\Debian\home\nir\voipappz\app"), want);
        assert_eq!(location(r"\\wsl.localhost\Ubuntu"), Some(("Ubuntu".into(), "/".into())));
    }

    #[test]
    fn other_paths_are_not_wsl() {
        assert_eq!(location(r"C:\Users\nir\voipappz-app"), None);
        assert_eq!(location(r"\\server\share\app"), None);
        assert_eq!(location("/home/nir/voipappz/app"), None);
        assert_eq!(location(r"\\wsl.localhost\"), None);
    }

    #[test]
    fn unc_round_trips() {
        let path = unc("Debian", "/home/nir/voipappz/app-builder/src/App.jsx");
        assert_eq!(path, r"\\wsl.localhost\Debian\home\nir\voipappz\app-builder\src\App.jsx");
        assert_eq!(location(&path), Some(("Debian".into(), "/home/nir/voipappz/app-builder/src/App.jsx".into())));
    }
}
