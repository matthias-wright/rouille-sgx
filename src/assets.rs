// Copyright (c) 2016 The Rouille developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::fs;
use std::path::Path;

#[cfg(not(target_env = "sgx"))]
use filetime;

#[cfg(not(target_env = "sgx"))]
use time;

use crate::Request;
use crate::Response;

/// Searches inside `path` for a file that matches the given request. If a file is found,
/// returns a `Response` that would serve this file if returned. If no file is found, a 404
/// response is returned instead.
///
/// The value of the `Content-Type` header of the response is guessed based on the file's
/// extension. If you wish so, you can modify that `Content-Type` by modifying the `Response`
/// object returned by this function.
///
/// # Example
///
/// In this example, a request made for example to `/test.txt` will return the file
/// `public/test.txt` (relative to the current working directory, which is usually the location
/// of the `Cargo.toml`) if it exists.
///
/// ```no_run
/// rouille::start_server("localhost:8000", move |request| {
///     let response = rouille::match_assets(&request, "public");
///     if response.is_success() {
///         return response;
///     }
///
///     // ...
///     # panic!()
/// });
/// ```
///
/// # Security
///
/// Everything inside the directory that you pass as `path` is potentially accessible by any
/// client. **Do not use assume that client won't be able to guess the URL of a sensitive file**.
/// All sensitive files should require a login/password to be accessed.
///
/// If you want to serve sensitive files, you are encouraged to put them in a different directory
/// than public files, and call `match_assets` once for public files and once for private files
/// after you checked the user's credentials.
/// Only call `match_assets` **after** you know that the user can have access to all the files
/// that can be served.
///
/// If you manage the user's accesses per-file, use a white list of authorized files instead of a
/// black list of forbidden files. Files can potentially be accessed from multiple different URLs
/// and a black list may not cover everything.
///
/// # Example with prefix
///
/// Sometimes you want to add a prefix to the URL of your static files. To do that, you can use
/// the `remove_prefix` method on `Request`.
///
/// ```no_run
/// rouille::start_server("localhost:8000", move |request| {
///     if let Some(request) = request.remove_prefix("/static") {
///         return rouille::match_assets(&request, "public");
///     }
///
///     // ...
///     # panic!()
/// });
/// ```
///
/// In this example, a request made to `/static/test.txt` will return the file
/// `public/test.txt` if it exists.
///
#[cfg(not(target_env = "sgx"))]
pub fn match_assets<P: ?Sized>(request: &Request, path: &P) -> Response
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Response::empty_404(),
    };

    // The potential location of the file on the disk.
    let potential_file = {
        // Clippy erroneously identifies this transform as a redundant clone
        #[allow(clippy::redundant_clone)]
        let mut path = path.to_path_buf();
        for component in request.url().split('/') {
            path.push(component);
        }
        path
    };

    // We try to canonicalize the file. If this fails, then the file doesn't exist.
    let potential_file = match potential_file.canonicalize() {
        Ok(f) => f,
        Err(_) => return Response::empty_404(),
    };

    // Check that we're still within `path`. This should eliminate security issues with
    // requests like `GET /../private_file`.
    if !potential_file.starts_with(path) {
        return Response::empty_404();
    }

    // Check that it's a file and not a directory.
    match fs::metadata(&potential_file) {
        Ok(ref m) if m.is_file() => (),
        _ => return Response::empty_404(),
    };

    let extension = potential_file.extension().and_then(|s| s.to_str());

    let file = match fs::File::open(&potential_file) {
        Ok(f) => f,
        Err(_) => return Response::empty_404(),
    };

    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let etag: String = (fs::metadata(&potential_file)
        .map(|meta| filetime::FileTime::from_last_modification_time(&meta).unix_seconds() as u64)
        .unwrap_or(now.nanosecond() as u64)
        ^ 0xd3f4_0305_c9f8_e911_u64)
        .to_string();

    Response::from_file(extension_to_mime_impl(extension), file)
        .with_etag(request, etag)
        .with_public_cache(3600) // TODO: is this a good idea? what if the file is private?
}

#[cfg(target_env = "sgx")]
pub fn match_assets<P: ?Sized>(request: &Request, path: &P) -> Response
where
    P: AsRef<Path>,
{
    unimplemented!("unavailable in an sgx environment")
}

/// Returns the mime type of a file based on its extension, or `application/octet-stream` if the
/// extension is unknown.
#[inline]
pub fn extension_to_mime(extension: &str) -> &'static str {
    extension_to_mime_impl(Some(extension))
}

/// Returns the mime type of a file based on its extension.
fn extension_to_mime_impl(extension: Option<&str>) -> &'static str {
    // List taken from https://github.com/cybergeek94/mime_guess/blob/master/src/mime_types.rs,
    // itself taken from a dead link.
    match extension {
        Some("323") => "text/h323; charset=utf8",
        Some("3g2") => "video/3gpp2",
        Some("3gp") => "video/3gpp",
        Some("3gp2") => "video/3gpp2",
        Some("3gpp") => "video/3gpp",
        Some("7z") => "application/x-7z-compressed",
        Some("aa") => "audio/audible",
        Some("aac") => "audio/aac",
        Some("aaf") => "application/octet-stream",
        Some("aax") => "audio/vnd.audible.aax",
        Some("ac3") => "audio/ac3",
        Some("aca") => "application/octet-stream",
        Some("accda") => "application/msaccess.addin",
        Some("accdb") => "application/msaccess",
        Some("accdc") => "application/msaccess.cab",
        Some("accde") => "application/msaccess",
        Some("accdr") => "application/msaccess.runtime",
        Some("accdt") => "application/msaccess",
        Some("accdw") => "application/msaccess.webapplication",
        Some("accft") => "application/msaccess.ftemplate",
        Some("acx") => "application/internet-property-stream",
        Some("addin") => "application/xml",
        Some("ade") => "application/msaccess",
        Some("adobebridge") => "application/x-bridge-url",
        Some("adp") => "application/msaccess",
        Some("adt") => "audio/vnd.dlna.adts",
        Some("adts") => "audio/aac",
        Some("afm") => "application/octet-stream",
        Some("ai") => "application/postscript",
        Some("aif") => "audio/x-aiff",
        Some("aifc") => "audio/aiff",
        Some("aiff") => "audio/aiff",
        Some("air") => "application/vnd.adobe.air-application-installer-package+zip",
        Some("amc") => "application/x-mpeg",
        Some("application") => "application/x-ms-application",
        Some("art") => "image/x-jg",
        Some("asa") => "application/xml",
        Some("asax") => "application/xml",
        Some("ascx") => "application/xml",
        Some("asd") => "application/octet-stream",
        Some("asf") => "video/x-ms-asf",
        Some("ashx") => "application/xml",
        Some("asi") => "application/octet-stream",
        Some("asm") => "text/plain; charset=utf8",
        Some("asmx") => "application/xml",
        Some("aspx") => "application/xml",
        Some("asr") => "video/x-ms-asf",
        Some("asx") => "video/x-ms-asf",
        Some("atom") => "application/atom+xml",
        Some("au") => "audio/basic",
        Some("avi") => "video/x-msvideo",
        Some("axs") => "application/olescript",
        Some("bas") => "text/plain; charset=utf8",
        Some("bcpio") => "application/x-bcpio",
        Some("bin") => "application/octet-stream",
        Some("bmp") => "image/bmp",
        Some("c") => "text/plain; charset=utf8",
        Some("cab") => "application/octet-stream",
        Some("caf") => "audio/x-caf",
        Some("calx") => "application/vnd.ms-office.calx",
        Some("cat") => "application/vnd.ms-pki.seccat",
        Some("cc") => "text/plain; charset=utf8",
        Some("cd") => "text/plain; charset=utf8",
        Some("cdda") => "audio/aiff",
        Some("cdf") => "application/x-cdf",
        Some("cer") => "application/x-x509-ca-cert",
        Some("chm") => "application/octet-stream",
        Some("class") => "application/x-java-applet",
        Some("clp") => "application/x-msclip",
        Some("cmx") => "image/x-cmx",
        Some("cnf") => "text/plain; charset=utf8",
        Some("cod") => "image/cis-cod",
        Some("config") => "application/xml",
        Some("contact") => "text/x-ms-contact; charset=utf8",
        Some("coverage") => "application/xml",
        Some("cpio") => "application/x-cpio",
        Some("cpp") => "text/plain; charset=utf8",
        Some("crd") => "application/x-mscardfile",
        Some("crl") => "application/pkix-crl",
        Some("crt") => "application/x-x509-ca-cert",
        Some("cs") => "text/plain; charset=utf8",
        Some("csdproj") => "text/plain; charset=utf8",
        Some("csh") => "application/x-csh",
        Some("csproj") => "text/plain; charset=utf8",
        Some("css") => "text/css; charset=utf8",
        Some("csv") => "text/csv; charset=utf8",
        Some("cur") => "application/octet-stream",
        Some("cxx") => "text/plain; charset=utf8",
        Some("dat") => "application/octet-stream",
        Some("datasource") => "application/xml",
        Some("dbproj") => "text/plain; charset=utf8",
        Some("dcr") => "application/x-director",
        Some("def") => "text/plain; charset=utf8",
        Some("deploy") => "application/octet-stream",
        Some("der") => "application/x-x509-ca-cert",
        Some("dgml") => "application/xml",
        Some("dib") => "image/bmp",
        Some("dif") => "video/x-dv",
        Some("dir") => "application/x-director",
        Some("disco") => "application/xml",
        Some("dll") => "application/x-msdownload",
        Some("dll.config") => "application/xml",
        Some("dlm") => "text/dlm; charset=utf8",
        Some("doc") => "application/msword",
        Some("docm") => "application/vnd.ms-word.document.macroEnabled.12",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("dot") => "application/msword",
        Some("dotm") => "application/vnd.ms-word.template.macroEnabled.12",
        Some("dotx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
        Some("dsp") => "application/octet-stream",
        Some("dsw") => "text/plain; charset=utf8",
        Some("dtd") => "application/xml",
        Some("dtsConfig") => "application/xml",
        Some("dv") => "video/x-dv",
        Some("dvi") => "application/x-dvi",
        Some("dwf") => "drawing/x-dwf",
        Some("dwp") => "application/octet-stream",
        Some("dxr") => "application/x-director",
        Some("eml") => "message/rfc822",
        Some("emz") => "application/octet-stream",
        Some("eot") => "application/vnd.ms-fontobject",
        Some("eps") => "application/postscript",
        Some("etl") => "application/etl",
        Some("etx") => "text/x-setext; charset=utf8",
        Some("evy") => "application/envoy",
        Some("exe") => "application/octet-stream",
        Some("exe.config") => "application/xml",
        Some("fdf") => "application/vnd.fdf",
        Some("fif") => "application/fractals",
        Some("filters") => "Application/xml",
        Some("fla") => "application/octet-stream",
        Some("flr") => "x-world/x-vrml",
        Some("flv") => "video/x-flv",
        Some("fsscript") => "application/fsharp-script",
        Some("fsx") => "application/fsharp-script",
        Some("generictest") => "application/xml",
        Some("gif") => "image/gif",
        Some("group") => "text/x-ms-group; charset=utf8",
        Some("gsm") => "audio/x-gsm",
        Some("gtar") => "application/x-gtar",
        Some("gz") => "application/x-gzip",
        Some("h") => "text/plain; charset=utf8",
        Some("hdf") => "application/x-hdf",
        Some("hdml") => "text/x-hdml; charset=utf8",
        Some("hhc") => "application/x-oleobject",
        Some("hhk") => "application/octet-stream",
        Some("hhp") => "application/octet-stream",
        Some("hlp") => "application/winhlp",
        Some("hpp") => "text/plain; charset=utf8",
        Some("hqx") => "application/mac-binhex40",
        Some("hta") => "application/hta",
        Some("htc") => "text/x-component; charset=utf8",
        Some("htm") => "text/html; charset=utf8",
        Some("html") => "text/html; charset=utf8",
        Some("htt") => "text/webviewhtml; charset=utf8",
        Some("hxa") => "application/xml",
        Some("hxc") => "application/xml",
        Some("hxd") => "application/octet-stream",
        Some("hxe") => "application/xml",
        Some("hxf") => "application/xml",
        Some("hxh") => "application/octet-stream",
        Some("hxi") => "application/octet-stream",
        Some("hxk") => "application/xml",
        Some("hxq") => "application/octet-stream",
        Some("hxr") => "application/octet-stream",
        Some("hxs") => "application/octet-stream",
        Some("hxt") => "text/html; charset=utf8",
        Some("hxv") => "application/xml",
        Some("hxw") => "application/octet-stream",
        Some("hxx") => "text/plain; charset=utf8",
        Some("i") => "text/plain; charset=utf8",
        Some("ico") => "image/x-icon",
        Some("ics") => "application/octet-stream",
        Some("idl") => "text/plain; charset=utf8",
        Some("ief") => "image/ief",
        Some("iii") => "application/x-iphone",
        Some("inc") => "text/plain; charset=utf8",
        Some("inf") => "application/octet-stream",
        Some("inl") => "text/plain; charset=utf8",
        Some("ins") => "application/x-internet-signup",
        Some("ipa") => "application/x-itunes-ipa",
        Some("ipg") => "application/x-itunes-ipg",
        Some("ipproj") => "text/plain; charset=utf8",
        Some("ipsw") => "application/x-itunes-ipsw",
        Some("iqy") => "text/x-ms-iqy; charset=utf8",
        Some("isp") => "application/x-internet-signup",
        Some("ite") => "application/x-itunes-ite",
        Some("itlp") => "application/x-itunes-itlp",
        Some("itms") => "application/x-itunes-itms",
        Some("itpc") => "application/x-itunes-itpc",
        Some("ivf") => "video/x-ivf",
        Some("jar") => "application/java-archive",
        Some("java") => "application/octet-stream",
        Some("jck") => "application/liquidmotion",
        Some("jcz") => "application/liquidmotion",
        Some("jfif") => "image/pjpeg",
        Some("jnlp") => "application/x-java-jnlp-file",
        Some("jpb") => "application/octet-stream",
        Some("jpe") => "image/jpeg",
        Some("jpeg") => "image/jpeg",
        Some("jpg") => "image/jpeg",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("jsx") => "text/jscript; charset=utf8",
        Some("jsxbin") => "text/plain; charset=utf8",
        Some("latex") => "application/x-latex",
        Some("library-ms") => "application/windows-library+xml",
        Some("lit") => "application/x-ms-reader",
        Some("loadtest") => "application/xml",
        Some("lpk") => "application/octet-stream",
        Some("lsf") => "video/x-la-asf",
        Some("lst") => "text/plain; charset=utf8",
        Some("lsx") => "video/x-la-asf",
        Some("lzh") => "application/octet-stream",
        Some("m13") => "application/x-msmediaview",
        Some("m14") => "application/x-msmediaview",
        Some("m1v") => "video/mpeg",
        Some("m2t") => "video/vnd.dlna.mpeg-tts",
        Some("m2ts") => "video/vnd.dlna.mpeg-tts",
        Some("m2v") => "video/mpeg",
        Some("m3u") => "audio/x-mpegurl",
        Some("m3u8") => "audio/x-mpegurl",
        Some("m4a") => "audio/m4a",
        Some("m4b") => "audio/m4b",
        Some("m4p") => "audio/m4p",
        Some("m4r") => "audio/x-m4r",
        Some("m4v") => "video/x-m4v",
        Some("mac") => "image/x-macpaint",
        Some("mak") => "text/plain; charset=utf8",
        Some("man") => "application/x-troff-man",
        Some("manifest") => "application/x-ms-manifest",
        Some("map") => "text/plain; charset=utf8",
        Some("master") => "application/xml",
        Some("mda") => "application/msaccess",
        Some("mdb") => "application/x-msaccess",
        Some("mde") => "application/msaccess",
        Some("mdp") => "application/octet-stream",
        Some("me") => "application/x-troff-me",
        Some("mfp") => "application/x-shockwave-flash",
        Some("mht") => "message/rfc822",
        Some("mhtml") => "message/rfc822",
        Some("mid") => "audio/mid",
        Some("midi") => "audio/mid",
        Some("mix") => "application/octet-stream",
        Some("mk") => "text/plain; charset=utf8",
        Some("mmf") => "application/x-smaf",
        Some("mno") => "application/xml",
        Some("mny") => "application/x-msmoney",
        Some("mod") => "video/mpeg",
        Some("mov") => "video/quicktime",
        Some("movie") => "video/x-sgi-movie",
        Some("mp2") => "video/mpeg",
        Some("mp2v") => "video/mpeg",
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("mp4v") => "video/mp4",
        Some("mpa") => "video/mpeg",
        Some("mpe") => "video/mpeg",
        Some("mpeg") => "video/mpeg",
        Some("mpf") => "application/vnd.ms-mediapackage",
        Some("mpg") => "video/mpeg",
        Some("mpp") => "application/vnd.ms-project",
        Some("mpv2") => "video/mpeg",
        Some("mqv") => "video/quicktime",
        Some("ms") => "application/x-troff-ms",
        Some("msi") => "application/octet-stream",
        Some("mso") => "application/octet-stream",
        Some("mts") => "video/vnd.dlna.mpeg-tts",
        Some("mtx") => "application/xml",
        Some("mvb") => "application/x-msmediaview",
        Some("mvc") => "application/x-miva-compiled",
        Some("mxp") => "application/x-mmxp",
        Some("nc") => "application/x-netcdf",
        Some("nsc") => "video/x-ms-asf",
        Some("nws") => "message/rfc822",
        Some("ocx") => "application/octet-stream",
        Some("oda") => "application/oda",
        Some("odc") => "text/x-ms-odc; charset=utf8",
        Some("odh") => "text/plain; charset=utf8",
        Some("odl") => "text/plain; charset=utf8",
        Some("odp") => "application/vnd.oasis.opendocument.presentation",
        Some("ods") => "application/oleobject",
        Some("odt") => "application/vnd.oasis.opendocument.text",
        Some("ogg") => "application/ogg",
        Some("one") => "application/onenote",
        Some("onea") => "application/onenote",
        Some("onepkg") => "application/onenote",
        Some("onetmp") => "application/onenote",
        Some("onetoc") => "application/onenote",
        Some("onetoc2") => "application/onenote",
        Some("orderedtest") => "application/xml",
        Some("osdx") => "application/opensearchdescription+xml",
        Some("otf") => "application/x-font-opentype",
        Some("p10") => "application/pkcs10",
        Some("p12") => "application/x-pkcs12",
        Some("p7b") => "application/x-pkcs7-certificates",
        Some("p7c") => "application/pkcs7-mime",
        Some("p7m") => "application/pkcs7-mime",
        Some("p7r") => "application/x-pkcs7-certreqresp",
        Some("p7s") => "application/pkcs7-signature",
        Some("pbm") => "image/x-portable-bitmap",
        Some("pcast") => "application/x-podcast",
        Some("pct") => "image/pict",
        Some("pcx") => "application/octet-stream",
        Some("pcz") => "application/octet-stream",
        Some("pdf") => "application/pdf",
        Some("pfb") => "application/octet-stream",
        Some("pfm") => "application/octet-stream",
        Some("pfx") => "application/x-pkcs12",
        Some("pgm") => "image/x-portable-graymap",
        Some("pic") => "image/pict",
        Some("pict") => "image/pict",
        Some("pkgdef") => "text/plain; charset=utf8",
        Some("pkgundef") => "text/plain; charset=utf8",
        Some("pko") => "application/vnd.ms-pki.pko",
        Some("pls") => "audio/scpls",
        Some("pma") => "application/x-perfmon",
        Some("pmc") => "application/x-perfmon",
        Some("pml") => "application/x-perfmon",
        Some("pmr") => "application/x-perfmon",
        Some("pmw") => "application/x-perfmon",
        Some("png") => "image/png",
        Some("pnm") => "image/x-portable-anymap",
        Some("pnt") => "image/x-macpaint",
        Some("pntg") => "image/x-macpaint",
        Some("pnz") => "image/png",
        Some("pot") => "application/vnd.ms-powerpoint",
        Some("potm") => "application/vnd.ms-powerpoint.template.macroEnabled.12",
        Some("potx") => "application/vnd.openxmlformats-officedocument.presentationml.template",
        Some("ppa") => "application/vnd.ms-powerpoint",
        Some("ppam") => "application/vnd.ms-powerpoint.addin.macroEnabled.12",
        Some("ppm") => "image/x-portable-pixmap",
        Some("pps") => "application/vnd.ms-powerpoint",
        Some("ppsm") => "application/vnd.ms-powerpoint.slideshow.macroEnabled.12",
        Some("ppsx") => "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
        Some("ppt") => "application/vnd.ms-powerpoint",
        Some("pptm") => "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
        Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        Some("prf") => "application/pics-rules",
        Some("prm") => "application/octet-stream",
        Some("prx") => "application/octet-stream",
        Some("ps") => "application/postscript",
        Some("psc1") => "application/PowerShell",
        Some("psd") => "application/octet-stream",
        Some("psess") => "application/xml",
        Some("psm") => "application/octet-stream",
        Some("psp") => "application/octet-stream",
        Some("pub") => "application/x-mspublisher",
        Some("pwz") => "application/vnd.ms-powerpoint",
        Some("qht") => "text/x-html-insertion; charset=utf8",
        Some("qhtm") => "text/x-html-insertion; charset=utf8",
        Some("qt") => "video/quicktime",
        Some("qti") => "image/x-quicktime",
        Some("qtif") => "image/x-quicktime",
        Some("qtl") => "application/x-quicktimeplayer",
        Some("qxd") => "application/octet-stream",
        Some("ra") => "audio/x-pn-realaudio",
        Some("ram") => "audio/x-pn-realaudio",
        Some("rar") => "application/octet-stream",
        Some("ras") => "image/x-cmu-raster",
        Some("rat") => "application/rat-file",
        Some("rc") => "text/plain; charset=utf8",
        Some("rc2") => "text/plain; charset=utf8",
        Some("rct") => "text/plain; charset=utf8",
        Some("rdlc") => "application/xml",
        Some("resx") => "application/xml",
        Some("rf") => "image/vnd.rn-realflash",
        Some("rgb") => "image/x-rgb",
        Some("rgs") => "text/plain; charset=utf8",
        Some("rm") => "application/vnd.rn-realmedia",
        Some("rmi") => "audio/mid",
        Some("rmp") => "application/vnd.rn-rn_music_package",
        Some("roff") => "application/x-troff",
        Some("rpm") => "audio/x-pn-realaudio-plugin",
        Some("rqy") => "text/x-ms-rqy; charset=utf8",
        Some("rtf") => "application/rtf",
        Some("rtx") => "text/richtext; charset=utf8",
        Some("ruleset") => "application/xml",
        Some("s") => "text/plain; charset=utf8",
        Some("safariextz") => "application/x-safari-safariextz",
        Some("scd") => "application/x-msschedule",
        Some("sct") => "text/scriptlet; charset=utf8",
        Some("sd2") => "audio/x-sd2",
        Some("sdp") => "application/sdp",
        Some("sea") => "application/octet-stream",
        Some("searchConnector-ms") => "application/windows-search-connector+xml",
        Some("setpay") => "application/set-payment-initiation",
        Some("setreg") => "application/set-registration-initiation",
        Some("settings") => "application/xml",
        Some("sfnt") => "application/font-sfnt",
        Some("sgimb") => "application/x-sgimb",
        Some("sgml") => "text/sgml; charset=utf8",
        Some("sh") => "application/x-sh",
        Some("shar") => "application/x-shar",
        Some("shtml") => "text/html; charset=utf8",
        Some("sit") => "application/x-stuffit",
        Some("sitemap") => "application/xml",
        Some("skin") => "application/xml",
        Some("sldm") => "application/vnd.ms-powerpoint.slide.macroEnabled.12",
        Some("sldx") => "application/vnd.openxmlformats-officedocument.presentationml.slide",
        Some("slk") => "application/vnd.ms-excel",
        Some("sln") => "text/plain; charset=utf8",
        Some("slupkg-ms") => "application/x-ms-license",
        Some("smd") => "audio/x-smd",
        Some("smi") => "application/octet-stream",
        Some("smx") => "audio/x-smd",
        Some("smz") => "audio/x-smd",
        Some("snd") => "audio/basic",
        Some("snippet") => "application/xml",
        Some("snp") => "application/octet-stream",
        Some("sol") => "text/plain; charset=utf8",
        Some("sor") => "text/plain; charset=utf8",
        Some("spc") => "application/x-pkcs7-certificates",
        Some("spl") => "application/futuresplash",
        Some("src") => "application/x-wais-source",
        Some("srf") => "text/plain; charset=utf8",
        Some("ssisdeploymentmanifest") => "application/xml",
        Some("ssm") => "application/streamingmedia",
        Some("sst") => "application/vnd.ms-pki.certstore",
        Some("stl") => "application/vnd.ms-pki.stl",
        Some("sv4cpio") => "application/x-sv4cpio",
        Some("sv4crc") => "application/x-sv4crc",
        Some("svc") => "application/xml",
        Some("svg") => "image/svg+xml",
        Some("swf") => "application/x-shockwave-flash",
        Some("t") => "application/x-troff",
        Some("tar") => "application/x-tar",
        Some("tcl") => "application/x-tcl",
        Some("testrunconfig") => "application/xml",
        Some("testsettings") => "application/xml",
        Some("tex") => "application/x-tex",
        Some("texi") => "application/x-texinfo",
        Some("texinfo") => "application/x-texinfo",
        Some("tgz") => "application/x-compressed",
        Some("thmx") => "application/vnd.ms-officetheme",
        Some("thn") => "application/octet-stream",
        Some("tif") => "image/tiff",
        Some("tiff") => "image/tiff",
        Some("tlh") => "text/plain; charset=utf8",
        Some("tli") => "text/plain; charset=utf8",
        Some("toc") => "application/octet-stream",
        Some("tr") => "application/x-troff",
        Some("trm") => "application/x-msterminal",
        Some("trx") => "application/xml",
        Some("ts") => "video/vnd.dlna.mpeg-tts",
        Some("tsv") => "text/tab-separated-values; charset=utf8",
        Some("ttf") => "application/x-font-ttf",
        Some("tts") => "video/vnd.dlna.mpeg-tts",
        Some("txt") => "text/plain; charset=utf8",
        Some("u32") => "application/octet-stream",
        Some("uls") => "text/iuls; charset=utf8",
        Some("user") => "text/plain; charset=utf8",
        Some("ustar") => "application/x-ustar",
        Some("vb") => "text/plain; charset=utf8",
        Some("vbdproj") => "text/plain; charset=utf8",
        Some("vbk") => "video/mpeg",
        Some("vbproj") => "text/plain; charset=utf8",
        Some("vbs") => "text/vbscript; charset=utf8",
        Some("vcf") => "text/x-vcard; charset=utf8",
        Some("vcproj") => "Application/xml",
        Some("vcs") => "text/plain; charset=utf8",
        Some("vcxproj") => "Application/xml",
        Some("vddproj") => "text/plain; charset=utf8",
        Some("vdp") => "text/plain; charset=utf8",
        Some("vdproj") => "text/plain; charset=utf8",
        Some("vdx") => "application/vnd.ms-visio.viewer",
        Some("vml") => "application/xml",
        Some("vscontent") => "application/xml",
        Some("vsct") => "application/xml",
        Some("vsd") => "application/vnd.visio",
        Some("vsi") => "application/ms-vsi",
        Some("vsix") => "application/vsix",
        Some("vsixlangpack") => "application/xml",
        Some("vsixmanifest") => "application/xml",
        Some("vsmdi") => "application/xml",
        Some("vspscc") => "text/plain; charset=utf8",
        Some("vss") => "application/vnd.visio",
        Some("vsscc") => "text/plain; charset=utf8",
        Some("vssettings") => "application/xml",
        Some("vssscc") => "text/plain; charset=utf8",
        Some("vst") => "application/vnd.visio",
        Some("vstemplate") => "application/xml",
        Some("vsto") => "application/x-ms-vsto",
        Some("vsw") => "application/vnd.visio",
        Some("vsx") => "application/vnd.visio",
        Some("vtx") => "application/vnd.visio",
        Some("wasm") => "application/wasm",
        Some("wav") => "audio/wav",
        Some("wave") => "audio/wav",
        Some("wax") => "audio/x-ms-wax",
        Some("wbk") => "application/msword",
        Some("wbmp") => "image/vnd.wap.wbmp",
        Some("wcm") => "application/vnd.ms-works",
        Some("wdb") => "application/vnd.ms-works",
        Some("wdp") => "image/vnd.ms-photo",
        Some("webarchive") => "application/x-safari-webarchive",
        Some("webtest") => "application/xml",
        Some("wiq") => "application/xml",
        Some("wiz") => "application/msword",
        Some("wks") => "application/vnd.ms-works",
        Some("wlmp") => "application/wlmoviemaker",
        Some("wlpginstall") => "application/x-wlpg-detect",
        Some("wlpginstall3") => "application/x-wlpg3-detect",
        Some("wm") => "video/x-ms-wm",
        Some("wma") => "audio/x-ms-wma",
        Some("wmd") => "application/x-ms-wmd",
        Some("wmf") => "application/x-msmetafile",
        Some("wml") => "text/vnd.wap.wml; charset=utf8",
        Some("wmlc") => "application/vnd.wap.wmlc",
        Some("wmls") => "text/vnd.wap.wmlscript; charset=utf8",
        Some("wmlsc") => "application/vnd.wap.wmlscriptc",
        Some("wmp") => "video/x-ms-wmp",
        Some("wmv") => "video/x-ms-wmv",
        Some("wmx") => "video/x-ms-wmx",
        Some("wmz") => "application/x-ms-wmz",
        Some("woff") => "application/font-woff",
        Some("woff2") => "application/font-woff2",
        Some("wpl") => "application/vnd.ms-wpl",
        Some("wps") => "application/vnd.ms-works",
        Some("wri") => "application/x-mswrite",
        Some("wrl") => "x-world/x-vrml",
        Some("wrz") => "x-world/x-vrml",
        Some("wsc") => "text/scriptlet; charset=utf8",
        Some("wsdl") => "application/xml",
        Some("wvx") => "video/x-ms-wvx",
        Some("x") => "application/directx",
        Some("xaf") => "x-world/x-vrml",
        Some("xaml") => "application/xaml+xml",
        Some("xap") => "application/x-silverlight-app",
        Some("xbap") => "application/x-ms-xbap",
        Some("xbm") => "image/x-xbitmap",
        Some("xdr") => "text/plain; charset=utf8",
        Some("xht") => "application/xhtml+xml",
        Some("xhtml") => "application/xhtml+xml",
        Some("xla") => "application/vnd.ms-excel",
        Some("xlam") => "application/vnd.ms-excel.addin.macroEnabled.12",
        Some("xlc") => "application/vnd.ms-excel",
        Some("xld") => "application/vnd.ms-excel",
        Some("xlk") => "application/vnd.ms-excel",
        Some("xll") => "application/vnd.ms-excel",
        Some("xlm") => "application/vnd.ms-excel",
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsb") => "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
        Some("xlsm") => "application/vnd.ms-excel.sheet.macroEnabled.12",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("xlt") => "application/vnd.ms-excel",
        Some("xltm") => "application/vnd.ms-excel.template.macroEnabled.12",
        Some("xltx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
        Some("xlw") => "application/vnd.ms-excel",
        Some("xml") => "application/xml",
        Some("xmta") => "application/xml",
        Some("xof") => "x-world/x-vrml",
        Some("xoml") => "text/plain; charset=utf8",
        Some("xpm") => "image/x-xpixmap",
        Some("xps") => "application/vnd.ms-xpsdocument",
        Some("xrm-ms") => "application/xml",
        Some("xsc") => "application/xml",
        Some("xsd") => "application/xml",
        Some("xsf") => "application/xml",
        Some("xsl") => "application/xml",
        Some("xslt") => "application/xslt+xml",
        Some("xsn") => "application/octet-stream",
        Some("xss") => "application/xml",
        Some("xtp") => "application/octet-stream",
        Some("xwd") => "image/x-xwindowdump",
        Some("z") => "application/x-compress",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
}
