extern crate cc;
extern crate ctest;

use std::env;
use std::str;

fn main() {
    let mut cfg = ctest::TestGenerator::new();

    let mut build = cc::Build::new();
    build.file("version_detect.c");
    if let Ok(out) = env::var("DEP_CURL_INCLUDE") {
        cfg.include(&out);
        build.include(&out);
    }
    let version = build.expand();
    let version = str::from_utf8(&version).unwrap();
    let version = version
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("#"))
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(10000);

    if env::var("TARGET").unwrap().contains("msvc") {
        cfg.flag("/wd4574"); // did you mean to use '#if INCL_WINSOCK_API_TYPEDEFS'
    }

    cfg.header("curl/curl.h");
    cfg.define("CURL_STATICLIB", None);
    cfg.field_name(|s, field| {
        if s == "curl_fileinfo" {
            field.replace("strings_", "strings.")
        } else if s == "CURLMsg" && field == "data" {
            "data.whatever".to_string()
        } else {
            field.to_string()
        }
    });
    cfg.type_name(|s, is_struct, _is_union| match s {
        "CURL" | "CURLM" | "CURLSH" | "curl_version_info_data" => s.to_string(),
        "curl_khtype" | "curl_khstat" | "curl_khmatch" => format!("enum {}", s),
        s if is_struct => format!("struct {}", s),
        "sockaddr" => format!("struct sockaddr"),
        s => s.to_string(),
    });
    // cfg.fn_cname(|s, l| l.unwrap_or(s).to_string());
    cfg.skip_type(|n| n == "__enum_ty");
    cfg.skip_signededness(|s| s.ends_with("callback") || s.ends_with("function"));

    cfg.skip_struct(move |s| {
        if version < 65 {
            match s {
                "curl_version_info_data" => return true,
                _ => {}
            }
        }

        false
    });

    cfg.skip_const(move |s| {
        if version < 64 {
            match s {
                "CURLE_HTTP2" => return true,
                "CURLE_PEER_FAILED_VERIFICATION" => return true,
                "CURLE_NO_CONNECTION_AVAILABLE" => return true,
                "CURLE_SSL_PINNEDPUBKEYNOTMATCH" => return true,
                "CURLE_SSL_INVALIDCERTSTATUS" => return true,
                "CURLE_HTTP2_STREAM" => return true,
                "CURLE_RECURSIVE_API_CALL" => return true,
                _ => {}
            }
        }
        if version < 61 {
            match s {
                "CURLOPT_PIPEWAIT" => return true,
                "CURLE_PEER_FAILED_VERIFICATION" => return true,
                _ => {}
            }
        }
        if version < 60 {
            match s {
                "CURLVERSION_FIFTH" | "CURLVERSION_SIXTH" | "CURLVERSION_NOW" => return true,
                _ => {}
            }
        }
        if version < 54 {
            match s {
                "CURL_SSLVERSION_TLSv1_3" => return true,
                _ => {}
            }
        }

        if version < 49 {
            if s.starts_with("CURL_HTTP_VERSION_2_PRIOR_KNOWLEDGE") {
                return true;
            }
        }

        if version < 47 {
            if s.starts_with("CURL_HTTP_VERSION_2") {
                return true;
            }
        }

        if version < 43 {
            if s.starts_with("CURLPIPE_") {
                return true;
            }
        }

        // OSX doesn't have this yet
        s == "CURLSSLOPT_NO_REVOKE" ||

        // A lot of curl versions doesn't support unix sockets
        s == "CURLOPT_UNIX_SOCKET_PATH" || s == "CURL_VERSION_UNIX_SOCKETS"
    });

    if cfg!(target_env = "msvc") {
        cfg.skip_fn_ptrcheck(|s| s.starts_with("curl_"));
    }

    cfg.generate("../curl-sys/lib.rs", "all.rs");
}
