use fern::colors::Color;

// Common
pub const AF: &str = "af";

// Env vars
pub const XPC_SERVICE_NAME: &str = "XPC_SERVICE_NAME";
pub const HOME: &str = "HOME";
pub const HOMEBREW_PREFIX: &str = "HOMEBREW_PREFIX";
pub const PATH: &str = "PATH";
pub const TERM_PROGRAM: &str = "TERM_PROGRAM";

// Languages
pub const C: &str = "c";
pub const CPP: &str = "c++";
pub const GO: &str = "go";
pub const JS: &str = "javascript";
pub const RUBY: &str = "ruby";
pub const RUST: &str = "rust";

// IDEs
pub const CLION: &str = "clion";
pub const GOLAND: &str = "goland";
pub const RUBYMINE: &str = "rubymine";
pub const RUSTROVER: &str = "rustrover";
pub const WEBSTORM: &str = "webstorm";
pub const ZED: &str = "zed";
pub const CODE: &str = "code";
pub const CODE_INSIDERS: &str = "code-insiders";

// Commands
pub const WHICH: &str = "which";
pub const GIT: &str = "git";
pub const PBCOPY: &str = "pbcopy";
pub const BREW: &str = "brew";

// Flags
pub const FLAG_VERSION: &str = "--version";
pub const FLAG_PREFIX: &str = "--prefix";
pub const FLAG_URL: &str = "--url";
pub const FLAG_NEW_TAB: &str = "--new-tab";

// Git specific
pub const HEAD: &str = "HEAD";
pub const ORIGIN: &str = "origin";
pub const UPSTREAM: &str = "upstream";
pub const ORIGIN_SLICE: &[&str] = &[ORIGIN];
pub const UPSTREAM_SLICE: &[&str] = &[UPSTREAM];
pub const UPSTREAM_ORIGIN_SLICE: &[&str] = &[UPSTREAM, ORIGIN];
pub const ORIGIN_UPSTREAM_SLICE: &[&str] = &[ORIGIN, UPSTREAM];
pub const PUSH: &str = "push";
pub const FETCH: &str = "fetch";
pub const MERGE: &str = "merge";
pub const CHECKOUT: &str = "checkout";
pub const DIFF: &str = "diff";
pub const NO_VERIFY: &str = "--no-verify";
pub const FORCE_WITH_LEASE: &str = "--force-with-lease";
pub const FF_ONLY: &str = "--ff-only";
pub const FORCE: &str = "--force";

// Colors
pub const GREY: Color = Color::TrueColor {
    r: 107,
    g: 107,
    b: 107,
};
pub const MUTED_TEAL: Color = Color::TrueColor {
    r: 117,
    g: 195,
    b: 170,
};

// Misc
pub const DEFAULT_HOMEBREW_PREFIX: &str = "/opt/homebrew";
