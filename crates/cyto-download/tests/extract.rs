use std::fs;
use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;
use tempfile::TempDir;

fn make_tarball(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = tar::Builder::new(&mut encoder);
        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *content).unwrap();
        }
        builder.finish().unwrap();
    }
    encoder.finish().unwrap()
}

#[test]
fn extracts_files_with_top_level_stripped() {
    let tarball = make_tarball(&[
        ("resources/foo.txt", b"hello"),
        ("resources/bar.txt", b"world"),
    ]);

    let dest = TempDir::new().unwrap();
    cyto_download::extract_tarball(&tarball, dest.path()).unwrap();

    assert_eq!(
        fs::read_to_string(dest.path().join("foo.txt")).unwrap(),
        "hello"
    );
    assert_eq!(
        fs::read_to_string(dest.path().join("bar.txt")).unwrap(),
        "world"
    );
}

#[test]
fn skips_apple_double_files() {
    let tarball = make_tarball(&[
        ("resources/foo.txt", b"real"),
        ("resources/._foo.txt", b"apple double junk"),
    ]);

    let dest = TempDir::new().unwrap();
    cyto_download::extract_tarball(&tarball, dest.path()).unwrap();

    assert!(dest.path().join("foo.txt").exists());
    assert!(!dest.path().join("._foo.txt").exists());
}

#[test]
fn handles_nested_directories() {
    let tarball = make_tarball(&[("resources/subdir/nested.txt", b"deep")]);

    let dest = TempDir::new().unwrap();
    cyto_download::extract_tarball(&tarball, dest.path()).unwrap();

    assert_eq!(
        fs::read_to_string(dest.path().join("subdir/nested.txt")).unwrap(),
        "deep"
    );
}

#[test]
fn skips_empty_after_stripping() {
    // Top-level directory entry only, no files
    let tarball = make_tarball(&[("resources/real.txt", b"content")]);

    let dest = TempDir::new().unwrap();
    cyto_download::extract_tarball(&tarball, dest.path()).unwrap();

    assert_eq!(
        fs::read_to_string(dest.path().join("real.txt")).unwrap(),
        "content"
    );
}

#[test]
fn run_skips_when_version_matches() {
    let dest = TempDir::new().unwrap();
    let output = dest.path().join("cyto");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join(".version"), "1.2.3").unwrap();

    let args = cyto_cli::download::ArgsDownload {
        output: Some(output.clone()),
        force: false,
        version: Some("1.2.3".to_string()),
        url: None,
    };

    // Should return Ok without attempting a download
    cyto_download::run(&args, "0.0.0").unwrap();
}

#[test]
fn run_uses_server() {
    use std::net::TcpListener;
    use std::thread;

    let tarball = make_tarball(&[("resources/test.txt", b"from server")]);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n",
            tarball.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(&tarball).unwrap();
    });

    let dest = TempDir::new().unwrap();
    let output = dest.path().join("cyto");

    let args = cyto_cli::download::ArgsDownload {
        output: Some(output.clone()),
        force: false,
        version: Some("0.0.1".to_string()),
        url: Some(format!("http://127.0.0.1:{port}/cyto-resources.tar.gz")),
    };

    cyto_download::run(&args, "0.0.0").unwrap();

    assert_eq!(
        fs::read_to_string(output.join("test.txt")).unwrap(),
        "from server"
    );
    assert_eq!(
        fs::read_to_string(output.join(".version")).unwrap(),
        "0.0.1"
    );

    handle.join().unwrap();
}
