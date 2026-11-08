//! `handler-drift`: an MSW or nock handler whose pattern can never match a URL the
//! production code emits, so the handler is dead and a per-spec override hides it.
//!
//! For each handler the rule folds the base URLs production emits (the `urlSources` config
//! names them: a module export, `path#name` for a single one, or a literal base) and asks
//! whether any base URL's path can prefix a URL the handler pattern matches. A pattern is
//! *drifted* when no folded base URL can prefix any URL it matches.
//!
//! # `*` semantics, as MSW 2 defines them
//!
//! MSW's `matchRequestUrl` rewrites every `*` in a pattern to `(.*)` before handing it to
//! `path-to-regexp`, and matches the result against `origin + pathname` (query and hash
//! dropped). So a `*` matches any run of characters *including* `/`: a leading `*` swallows
//! the origin and any path prefix, and a `*` in the middle matches within and across
//! segments. `:name` is one segment. A pattern with no scheme and host is matched against
//! the pathname only. The rule reads a pattern the same way:
//!
//! | pattern                          | base                          | outcome    | why |
//! |----------------------------------|-------------------------------|------------|-----|
//! | `*/v1/x`                         | `http://h/v1`                 | reachable  | the `*` is the origin, `/v1` aligns |
//! | `*/v1/x`                         | `http://h/api/v1`             | reachable  | the `*` swallows `/api` too |
//! | `*/x`                            | `http://h/v1`                 | reachable  | the `*` swallows the whole base path |
//! | `*/api/v1/x`                     | `http://h/v1`                 | drifted    | the tail spells `v1` behind `api`; the base has nothing before `v1` |
//! | `*/:version/x`                   | `http://h/v1`                 | reachable  | `:name` is one segment |
//! | `*/v1/*`                         | `http://h/v1/a/b`             | reachable  | a `*` in the middle crosses segments |
//! | `*`                              | anything                      | reachable  | matches every URL |
//! | `http://h/v1/x`                  | `http://h/v1`                 | reachable  | origin and path prefix agree |
//! | `http://h/api/v1/x`              | `http://h/v1`                 | drifted    | path |
//! | `http://other/v1/x`              | `http://h/v1`                 | drifted    | origin |
//! | `http://h/v*`                    | `http://h/v1/x`               | reachable  | a `*` inside a segment |
//! | `/v1/x`                          | `http://h/v1`                 | reachable  | pathname only |
//! | `/v1/x`                          | `http://h/api/v1`             | drifted    | pathname `/v1/x` does not start with `/api/v1` |
//! | `*/v1/x?q=1`                     | `http://h/v1`                 | reachable  | the query never takes part |
//! | `*/api/v1/x`                     | `${API}` (no `env`)           | undecided  | no base folded to a concrete value |
//! | `http://h/api/x`                 | `http://${HOST}/api` (no env) | undecided  | an opaque host, never a decided drift |
//! | a regex literal                  | anything                      | no row     | MSW's to run at runtime |
//! | a computed pattern               | anything                      | undecided  | could not be folded |
//!
//! Taken literally, a leading `*` can swallow the whole base path and every such pattern is
//! reachable from every base (`*/api/v1/x` does match `http://h/v1/api/v1/x`). The rule
//! keeps one refinement so the shipped class it was built on (a `BASE = "*/api/v1"`
//! against a `/v1` base) stays decidable: when the pattern's tail names the base path's last
//! segment, it takes that as the author spelling the base, and the segments before it must
//! then close the base path. A tail that never names it is an endpoint under a swallowed
//! base and is reachable. The refinement only ever turns "reachable" into "drifted" when
//! the pattern re-spells the base with a different prefix in front.
//!
//! # Honesty over a guess
//!
//! When the handler pattern is opaque (a computed URL), when no base URL folded at all, or
//! when every folded base still carries an opaque segment (a `process.env` read the config's
//! `env` assumptions did not resolve, in the host or in the path), the rule reports "could
//! not decide" and names what could not be folded. A decided drift requires at least one
//! concrete base URL the pattern cannot reach; an opaque base beside a concrete one keeps
//! the handler undecided whenever the opaque base could still reach it. The harness counts
//! undecided rows apart from findings, so a conservative rule is not scored as a false
//! positive.

use spirolo_model::{Constant, Handler, HandlerLibrary, Model, PatternKind};

use crate::finding::{
    number_collisions, sort, Evidence, Finding, Location, Outcome, RelatedLocation,
};
use crate::{Config, Family, Rule, Severity};

/// The `handler-drift` rule. See the module docs for the semantics.
pub struct HandlerDrift;

impl Rule for HandlerDrift {
    fn name(&self) -> &'static str {
        "handler-drift"
    }

    fn family(&self) -> Family {
        Family::Audit
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A mock handler's URL pattern disagrees with every URL the production code emits, so the handler can never match."
    }

    fn default_severity(&self) -> Severity {
        // Ships at `info` until a corpus proves it, the way CodeQL `@precision` and Semgrep
        // confidence tiers gate a young rule.
        Severity::Info
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let bases = base_urls(model, config);

        let mut findings: Vec<Finding> = model
            .handlers
            .iter()
            .filter_map(|handler| self.judge(handler, &bases, severity))
            .collect();

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl HandlerDrift {
    /// Judge one handler against the folded base URLs, or `None` when it neither drifts nor
    /// is worth an "undecided" row (it matches a base, so it is `ok`; or it is a regex the
    /// rule never judges).
    fn judge(&self, handler: &Handler, bases: &Bases, severity: Severity) -> Option<Finding> {
        // A regex-literal pattern is MSW's to run against the URL at runtime; the model
        // cannot, so it is recorded and never judged — never a finding, never undecided.
        if handler.pattern_kind == PatternKind::Regex {
            return None;
        }

        // A computed (opaque) pattern is honest "could not decide", not a guessed drift.
        let Some(pattern) = handler.pattern.as_deref() else {
            return Some(self.undecided(
                handler,
                severity,
                "the handler pattern is computed and could not be folded to a URL",
            ));
        };

        // No base URL folded at all — nothing to compare against — is undecided, not drift.
        if bases.concrete.is_empty() && bases.opaque.is_empty() {
            return Some(self.undecided(
                handler,
                severity,
                "no base URL folded from the configured urlSources",
            ));
        }

        let handler_url = HandlerPattern::parse(pattern);

        // A concrete base whose path prefixes the handler's path: the handler is reachable.
        if bases
            .concrete
            .iter()
            .any(|base| handler_url.reachable_from(&base.parsed))
        {
            return None;
        }

        // Every base is still opaque (a host or a segment the `env` assumptions did not
        // fill): there is no concrete base to drift against, so the row is undecided and
        // names the holes. A decided drift needs at least one folded base.
        if bases.concrete.is_empty() {
            let holes = bases.unresolved_holes();
            return Some(self.undecided(
                handler,
                severity,
                &format!("no base URL folded to a concrete value; unresolved: {holes}"),
            ));
        }

        // No concrete base reaches it, but an opaque base *might* once its holes are known:
        // stay honest rather than call a drift the env assumptions could dissolve.
        if bases
            .opaque
            .iter()
            .any(|base| handler_url.could_reach(&base.parsed))
        {
            return Some(self.undecided(
                handler,
                severity,
                &format!(
                    "a base URL has an opaque segment ({}); add an `env` assumption to decide it",
                    bases.unresolved_holes()
                ),
            ));
        }

        // At least one base is concrete and none reaches the handler: a decided drift.
        Some(self.drift(handler, bases, severity, pattern))
    }

    /// A decided-drift finding: the handler's pattern against the base URLs it cannot reach,
    /// naming both so a maintainer sees the disagreement without opening a file.
    fn drift(
        &self,
        handler: &Handler,
        bases: &Bases,
        severity: Severity,
        pattern: &str,
    ) -> Finding {
        let base_list = bases
            .concrete
            .iter()
            .map(|base| base.value.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let related = bases
            .concrete
            .iter()
            .map(|base| RelatedLocation {
                label: "base url".to_owned(),
                location: Location::at(base.file.clone(), base.span),
            })
            .collect();
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(handler.file.clone(), handler.span),
            related,
            message: format!("handler {pattern:?} matches no URL under any base ({base_list})"),
            evidence: vec![
                Evidence::new("base_urls", base_list.clone()),
                Evidence::new("library", library_name(handler.library)),
                Evidence::new("pattern", pattern.to_owned()),
            ],
            explain: format!(
                "The {} handler {pattern:?} can never match a request: its path is not \
                 reachable under any base URL production emits ({base_list}). Fix the \
                 handler's base path so it matches the folded production URL — do not delete \
                 the handler, the endpoint it mocks is still called.",
                library_name(handler.library),
            ),
            anchor: anchor_of(handler, pattern),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A could-not-decide row: recorded so a maintainer sees the rule looked, counted apart
    /// from findings so honesty is not scored as a false positive.
    fn undecided(&self, handler: &Handler, severity: Severity, why: &str) -> Finding {
        let pattern = handler
            .pattern
            .clone()
            .unwrap_or_else(|| "<computed>".to_owned());
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location::at(handler.file.clone(), handler.span),
            related: Vec::new(),
            message: format!("could not decide handler {pattern:?}: {why}"),
            evidence: vec![
                Evidence::new("library", library_name(handler.library)),
                Evidence::new("pattern", pattern.clone()),
                Evidence::new("reason", why.to_owned()),
            ],
            explain: format!(
                "The {} handler {pattern:?} could not be judged: {why}. Declare the base URL \
                 in `urlSources`, or its `${{VAR}}` segment's value in `env`, so the rule can \
                 decide whether the handler is reachable rather than guess.",
                library_name(handler.library),
            ),
            anchor: anchor_of(handler, &pattern),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }
}

/// The semantic anchor of a handler: its method and its pattern (`GET */v1/x`), so the
/// `GET` and the `POST` of one URL are two identities and the collision ordinal only ever
/// numbers a literal duplicate. `ANY` stands for MSW's `all` (no method).
fn anchor_of(handler: &Handler, pattern: &str) -> String {
    let method = handler.method.as_deref().unwrap_or("ANY");
    format!("{method} {pattern}")
}

/// The base URLs `urlSources` folded to, split by whether they are concrete or still carry
/// an opaque segment (a `${VAR}` hole the `env` config did not resolve).
#[derive(Default)]
struct Bases {
    /// Base URLs that folded to a concrete path.
    concrete: Vec<Base>,
    /// Base URLs still carrying an opaque segment, so a non-matching handler is undecided
    /// against them rather than drifted.
    opaque: Vec<Base>,
}

impl Bases {
    /// The `${VAR}` holes the opaque bases still carry, deduplicated and in first-seen
    /// order, for the undecided row's reason (`${NEXT_PUBLIC_API_URL}, ${API_VERSION}`).
    fn unresolved_holes(&self) -> String {
        let mut holes: Vec<String> = Vec::new();
        for base in &self.opaque {
            for hole in holes_in(&base.value) {
                if !holes.contains(&hole) {
                    holes.push(hole);
                }
            }
        }
        if holes.is_empty() {
            "an opaque value".to_owned()
        } else {
            holes.join(", ")
        }
    }
}

/// Every `${VAR}` hole in a folded value, as written, in order.
fn holes_in(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let after = &rest[start..];
        match after.find('}') {
            Some(end) => {
                out.push(after[..=end].to_owned());
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    out
}

/// One base URL a `urlSources` entry named, with where it was declared.
struct Base {
    /// The folded (env-substituted) value, for the message and the evidence.
    value: String,
    /// Repo-relative path of the module that declares it.
    file: String,
    /// Location of the constant.
    span: spirolo_model::Span,
    /// The parsed origin and path segments.
    parsed: BaseUrl,
}

/// Fold the base URLs the `urlSources` config names, applying `env` assumptions to any
/// `${VAR}` hole the model preserved on an opaque constant.
///
/// An entry is one of three forms: a literal base URL (`http://…`, `/v1`, or one with a
/// `${VAR}` hole) the maintainer declares directly, for a production base the model cannot
/// fold from source (a value assembled through a merge or a nested config object); a module
/// `path` (every folded string constant the module exports is a candidate base); or
/// `path#name` (only the constant `name`). A base that resolves to a concrete value is a
/// concrete base; one whose `${VAR}` hole survives the `env` substitution is an opaque base.
fn base_urls(model: &Model, config: &Config) -> Bases {
    let mut bases = Bases::default();
    for source in &config.url_sources {
        if is_literal_source(source) {
            // The maintainer declared the base directly; env-substitute its holes and record
            // it against the config file itself (there is no source constant to point at).
            let value = substitute_env(source, config);
            let opaque = value.contains("${");
            push_base(
                &mut bases,
                value,
                ".spirolorc.jsonc".to_owned(),
                spirolo_model::Span::default(),
                opaque,
            );
            continue;
        }
        let (path, name) = match source.split_once('#') {
            Some((path, name)) => (path, Some(name)),
            None => (source.as_str(), None),
        };
        let Some(module) = model.modules.iter().find(|module| module.path == path) else {
            continue;
        };
        for constant in &module.constants {
            if name.is_some_and(|name| name != constant.name) {
                continue;
            }
            if let Some((value, opaque)) = resolved_base(constant, config) {
                push_base(
                    &mut bases,
                    value,
                    module.path.clone(),
                    constant.span,
                    opaque,
                );
            }
        }
    }
    bases
}

/// Whether a `urlSources` entry is a literal base URL rather than a module path: it carries
/// a scheme (`http://`), starts with `/` (a root-relative base), or contains a `${VAR}` hole
/// the `env` config fills. A bare module path (`src/config/api.ts`) is none of these.
fn is_literal_source(source: &str) -> bool {
    source.contains("://") || source.starts_with('/') || source.contains("${")
}

/// Record one folded base, into the concrete or the opaque list.
fn push_base(
    bases: &mut Bases,
    value: String,
    file: String,
    span: spirolo_model::Span,
    opaque: bool,
) {
    let base = Base {
        parsed: BaseUrl::parse(&value),
        value,
        file,
        span,
    };
    if opaque {
        bases.opaque.push(base);
    } else {
        bases.concrete.push(base);
    }
}

/// A base URL from one constant, with a flag saying whether it still carries an opaque
/// `${VAR}` hole after applying the `env` assumptions; `None` when the constant carries no
/// URL at all (opaque for a non-env reason, or not a string).
fn resolved_base(constant: &Constant, config: &Config) -> Option<(String, bool)> {
    if let Some(value) = &constant.value {
        return Some((value.clone(), false));
    }
    let template = constant.env_template.as_ref()?;
    let substituted = substitute_env(template, config);
    let opaque = substituted.contains("${");
    Some((substituted, opaque))
}

/// Replace every `${VAR}` hole in `template` whose `VAR` the `env` config assumes with its
/// assumed value; leave the rest as `${VAR}`, which marks the base opaque.
fn substitute_env(template: &str, config: &Config) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let name = &after[..end];
            if let Some(value) = config.env.get(name) {
                out.push_str(value);
            } else {
                out.push_str("${");
                out.push_str(name);
                out.push('}');
            }
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// A folded base URL, parsed into an optional origin (scheme + host, for an absolute URL)
/// and its path segments. Query and fragment are dropped: MSW never matches on them.
struct BaseUrl {
    /// `scheme://host[:port]` for an absolute base; `None` for a relative one.
    origin: Option<String>,
    /// Path segments, empty runs dropped (`/v1/` and `/v1` parse the same).
    path: Vec<String>,
}

impl BaseUrl {
    /// Parse a folded base URL into its origin and path segments.
    fn parse(value: &str) -> Self {
        let value = drop_query(value);
        if let Some((scheme, rest)) = split_scheme(value) {
            // `host[:port]` is the run up to the first `/`; the rest is the path.
            let (host, path) = rest.split_once('/').map_or((rest, ""), |(h, p)| (h, p));
            Self {
                origin: Some(format!("{scheme}://{host}")),
                path: segments(path),
            }
        } else {
            Self {
                origin: None,
                path: segments(value.strip_prefix('/').unwrap_or(value)),
            }
        }
    }

    /// The base's leading segments up to its first `${VAR}` hole: what an opaque base is
    /// known to start with.
    fn known_path(&self) -> Vec<String> {
        self.path
            .iter()
            .take_while(|segment| !segment.contains("${"))
            .cloned()
            .collect()
    }

    /// Whether the base's origin is opaque: absent (a relative base) or still carrying a
    /// `${VAR}` hole, so it is compatible with any handler origin.
    fn origin_is_open(&self) -> bool {
        self.origin
            .as_deref()
            .is_none_or(|origin| origin.contains("${"))
    }
}

/// What part of a request URL a handler pattern is matched against, as MSW decides it
/// from the pattern's first characters.
#[derive(PartialEq)]
enum Scope {
    /// `scheme://host/…`: MSW matches origin and pathname. The origin may carry `*`.
    Absolute(String),
    /// `/…` (no scheme, no leading `*`): MSW matches the pathname only.
    Relative,
    /// `*…`: the leading `*` is `(.*)`, which swallows the origin and any path prefix.
    AnyPrefix,
}

/// A handler URL pattern, parsed into the scope MSW matches it in and its path *tokens*: a
/// literal segment, a `:name` (one segment), a `*` (any run of segments) or a segment with
/// a `*` inside. Query dropped.
struct HandlerPattern {
    /// What the pattern pins: an origin, the pathname, or nothing before its tail.
    scope: Scope,
    /// The path tokens (after the leading `*` for [`Scope::AnyPrefix`]).
    path: Vec<Token>,
}

/// One token of a handler pattern's path.
#[derive(PartialEq)]
enum Token {
    /// A literal segment that must match a base segment exactly.
    Literal(String),
    /// `:name` — matches exactly one segment of any value.
    Param,
    /// `*` — matches any run of characters, so any number of segments.
    Wildcard,
    /// A segment with a `*` inside (`v*`, `*.json`): the literal prefix before the `*` must
    /// start the base segment, and the `*` then absorbs the rest, segments included.
    Glob(String),
}

impl HandlerPattern {
    /// Parse a handler pattern into its scope and path tokens.
    fn parse(value: &str) -> Self {
        let value = drop_query(value);
        // A leading `*` is `(.*)`: any origin and any path prefix. Drop it and read the
        // rest as the pattern's tail.
        if let Some(rest) = value.strip_prefix('*') {
            return Self {
                scope: Scope::AnyPrefix,
                path: tokens(rest.trim_start_matches('*')),
            };
        }
        if let Some((scheme, rest)) = split_scheme(value) {
            let (host, path) = rest.split_once('/').map_or((rest, ""), |(h, p)| (h, p));
            return Self {
                scope: Scope::Absolute(format!("{scheme}://{host}")),
                path: tokens(path),
            };
        }
        Self {
            scope: Scope::Relative,
            path: tokens(value.strip_prefix('/').unwrap_or(value)),
        }
    }

    /// Whether a URL the handler matches is reachable under a concrete `base`, per the
    /// module's table.
    fn reachable_from(&self, base: &BaseUrl) -> bool {
        match &self.scope {
            Scope::Absolute(origin) => {
                origin_ok(origin, base.origin.as_deref()) && path_prefixes(&base.path, &self.path)
            }
            Scope::Relative => path_prefixes(&base.path, &self.path),
            Scope::AnyPrefix => any_prefix_reaches(&base.path, &self.path),
        }
    }

    /// Whether the handler *could* be reachable under an opaque `base` — used only for a base
    /// still carrying a `${VAR}` hole, so a non-matching handler is undecided rather than
    /// drifted. A hole in the origin is compatible with any handler origin; a hole in the
    /// path matches any handler token, so the check is over the base's known leading
    /// segments only. A leading `*` could always reach an opaque base.
    fn could_reach(&self, base: &BaseUrl) -> bool {
        match &self.scope {
            Scope::Absolute(origin) => {
                (base.origin_is_open() || origin_ok(origin, base.origin.as_deref()))
                    && path_prefixes(&base.known_path(), &self.path)
            }
            Scope::Relative => path_prefixes(&base.known_path(), &self.path),
            Scope::AnyPrefix => true,
        }
    }
}

/// Whether a handler origin pattern (which may carry `*`) is compatible with a base origin:
/// a relative base agrees on origin, because MSW resolves it against the same origin and
/// the pathname comparison then decides it.
fn origin_ok(handler: &str, base: Option<&str>) -> bool {
    base.is_none_or(|base| glob_matches(handler, base))
}

/// Whether `pattern`, with `*` as any run of characters, matches the whole of `text`.
fn glob_matches(pattern: &str, text: &str) -> bool {
    let Some((head, tail)) = pattern.split_once('*') else {
        return pattern == text;
    };
    let Some(rest) = text.strip_prefix(head) else {
        return false;
    };
    // The `*` may absorb any prefix of `rest`; try each split until the tail matches.
    (0..=rest.len())
        .filter(|&i| rest.is_char_boundary(i))
        .any(|i| glob_matches(tail, &rest[i..]))
}

/// Whether the concrete `base` path segments are a prefix of the handler `path` tokens,
/// honouring `*` (any run) and `:name` (one segment). This is the drift test for an
/// absolute or a relative pattern: a base path that prefixes a URL the handler matches
/// means the handler is reachable.
fn path_prefixes(base: &[String], path: &[Token]) -> bool {
    let mut b = 0;
    let mut p = 0;
    while b < base.len() {
        let Some(token) = path.get(p) else {
            // The handler path ran out before the base was consumed: a `*` tail could still
            // absorb the rest, but there is no token left, so the base is not a prefix.
            return false;
        };
        match token {
            Token::Wildcard => {
                // `*` absorbs any run: the base's remaining segments are all matched.
                return true;
            }
            Token::Glob(prefix) => {
                // `v*` needs this segment to start with `v`; the `*` then absorbs the rest.
                return base[b].starts_with(prefix.as_str());
            }
            Token::Param => {
                // `:name` matches this one base segment whatever its value.
                b += 1;
                p += 1;
            }
            Token::Literal(literal) => {
                if literal == &base[b] {
                    b += 1;
                    p += 1;
                } else {
                    return false;
                }
            }
        }
    }
    // Every base segment matched a handler token: the base prefixes the handler's path.
    true
}

/// The drift test for a pattern with a leading `*`: the `*` swallows the origin and any
/// prefix of the base path, so the pattern reaches the base when some suffix of the base
/// path is a prefix of the tail. When no suffix aligns, the `*` may still swallow the whole
/// base path, unless the tail names the base's last segment — then the author spelled the
/// base and the mismatch before it is a drift (see the module docs).
fn any_prefix_reaches(base: &[String], tail: &[Token]) -> bool {
    if (0..base.len()).any(|i| path_prefixes(&base[i..], tail)) {
        return true;
    }
    base.last().is_none_or(|last| {
        !tail
            .iter()
            .any(|token| matches!(token, Token::Literal(literal) if literal == last))
    })
}

/// Split a `scheme://…` URL into its scheme and the rest, `None` for a relative URL.
fn split_scheme(value: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = value.split_once("://")?;
    if scheme.is_empty() || scheme.contains('/') {
        None
    } else {
        Some((scheme, rest))
    }
}

/// Drop a query string and fragment: MSW never matches on them.
fn drop_query(value: &str) -> &str {
    let value = value.split('#').next().unwrap_or(value);
    value.split('?').next().unwrap_or(value)
}

/// A path's non-empty segments, split on `/`.
fn segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// A handler path's tokens: a `*` is a wildcard, a `:name` a one-segment param, a segment
/// with a `*` inside a glob, anything else a literal. Empty segments are dropped.
fn tokens(path: &str) -> Vec<Token> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            if segment.chars().all(|c| c == '*') {
                Token::Wildcard
            } else if segment.starts_with(':') {
                Token::Param
            } else if let Some((prefix, _)) = segment.split_once('*') {
                Token::Glob(prefix.to_owned())
            } else {
                Token::Literal(segment.to_owned())
            }
        })
        .collect()
}

/// The lower-case library name for a handler's evidence and message.
const fn library_name(library: HandlerLibrary) -> &'static str {
    match library {
        HandlerLibrary::Msw => "MSW",
        HandlerLibrary::Nock => "nock",
        HandlerLibrary::Unknown => "handler",
    }
}
