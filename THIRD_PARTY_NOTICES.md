# Third-party notices

The native binary is built from the Rust crates below, statically linked. The npm
launcher has no dependencies besides the platform packages carrying that binary.

Generated from `Cargo.lock`: `cargo tree -e normal --target all` lists every crate
compiled into the binary on any target, and `cargo metadata`
gives each one's licence and repository as declared in its own `Cargo.toml`. Workspace crates
(Apache-2.0, see [LICENSE](LICENSE)) and build- or dev-only dependencies are left out. The full licence
texts ship inside each crate's source package on crates.io.

| Crate | Version | Licence | Repository |
| --- | --- | --- | --- |
| aho-corasick | 1.1.5 | Unlicense OR MIT | https://github.com/BurntSushi/aho-corasick |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | https://github.com/zakarumych/allocator-api2 |
| anstream | 1.0.0 | MIT OR Apache-2.0 | https://github.com/rust-cli/anstyle.git |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | https://github.com/rust-cli/anstyle.git |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 | https://github.com/rust-cli/anstyle.git |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 | https://github.com/rust-cli/anstyle.git |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 | https://github.com/rust-cli/anstyle.git |
| arraydeque | 0.5.1 | MIT/Apache-2.0 | https://github.com/andylokandy/arraydeque |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | https://github.com/bluss/arrayvec |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags |
| bstr | 1.13.1 | MIT OR Apache-2.0 | https://github.com/BurntSushi/bstr |
| bytecount | 0.6.9 | Apache-2.0/MIT | https://github.com/llogiq/bytecount |
| byteorder | 1.5.0 | Unlicense OR MIT | https://github.com/BurntSushi/byteorder |
| castaway | 0.2.4 | MIT | https://github.com/sagebind/castaway |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if |
| clap | 4.6.7 | MIT OR Apache-2.0 | https://github.com/clap-rs/clap |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 | https://github.com/clap-rs/clap |
| clap_derive | 4.6.7 | MIT OR Apache-2.0 | https://github.com/clap-rs/clap |
| clap_lex | 1.1.0 | MIT OR Apache-2.0 | https://github.com/clap-rs/clap |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 | https://github.com/rust-cli/anstyle.git |
| compact_str | 0.10.0 | MIT | https://github.com/ParkMyCar/compact_str |
| concurrent_lru | 0.2.0 | MIT | https://github.com/ngkv/concurrent_lru.git |
| cow-utils | 0.1.3 | MIT | https://github.com/RReverser/cow-utils-rs |
| crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| dashmap | 6.2.1 | MIT | https://github.com/xacrimon/dashmap |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | https://github.com/yaahc/displaydoc |
| dragonbox_ecma | 0.1.12 | Apache-2.0 WITH LLVM-exception OR BSL-1.0 | https://github.com/magic-akari/dragonbox |
| either | 1.18.0 | MIT OR Apache-2.0 | https://github.com/rayon-rs/either |
| encoding_rs | 0.8.35 | (Apache-2.0 OR MIT) AND BSD-3-Clause | https://github.com/hsivonen/encoding_rs |
| endian-type | 0.2.0 | MIT | https://github.com/Lolirofle/endian-type.git |
| equivalent | 1.0.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| errno | 0.3.14 | MIT OR Apache-2.0 | https://github.com/lambda-fairy/rust-errno |
| fast-glob | 1.1.1 | MIT | https://github.com/oxc-project/fast-glob.git |
| fastrand | 2.5.0 | Apache-2.0 OR MIT | https://github.com/smol-rs/fastrand |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | https://github.com/rust-lang/flate2-rs |
| float-cmp | 0.10.0 | MIT | https://github.com/mikedilger/float-cmp |
| foldhash | 0.2.0 | Zlib | https://github.com/orlp/foldhash |
| globset | 0.4.21 | Unlicense OR MIT | https://github.com/BurntSushi/ripgrep/tree/master/crates/globset |
| halfbrown | 0.4.0 | Apache-2.0/MIT | https://github.com/Licenser/halfbrown |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| hashlink | 0.12.2 | MIT OR Apache-2.0 | https://github.com/djc/hashlink |
| heck | 0.5.0 | MIT OR Apache-2.0 | https://github.com/withoutboats/heck |
| icu_collections | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| icu_locale_core | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| icu_locale_fallback | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| icu_locale_fallback_data | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| icu_provider | 2.3.1 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| icu_segmenter | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| icu_segmenter_data | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| ignore | 0.4.34 | Unlicense OR MIT | https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 | https://github.com/polyfill-rs/is_terminal_polyfill |
| itoa | 1.0.18 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| json-strip-comments | 3.1.2 | Apache-2.0 | https://github.com/oxc-project/json-strip-comments |
| jsonc-parser | 0.34.0 | MIT | https://github.com/dprint/jsonc-parser |
| libc | 0.2.189 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/sunfishcode/linux-raw-sys |
| litemap | 0.8.3 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| log | 0.4.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/log |
| lsp-server | 0.10.0 | MIT OR Apache-2.0 | https://github.com/rust-lang/rust-analyzer/tree/master/lib/lsp-server |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| nibble_vec | 0.1.0 | MIT | https://github.com/michaelsproul/rust_nibble_vec |
| nodejs-built-in-modules | 1.0.0 | MIT | https://github.com/oxc-project/nodejs-built-in-modules |
| nonmax | 0.5.5 | MIT OR Apache-2.0 | https://github.com/LPGhatguy/nonmax |
| num-bigint | 0.5.1 | MIT OR Apache-2.0 | https://github.com/rust-num/num-bigint |
| num-integer | 0.1.47 | MIT OR Apache-2.0 | https://github.com/rust-num/num-integer |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | https://github.com/rust-num/num-traits |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | https://github.com/matklad/once_cell |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 | https://github.com/polyfill-rs/once_cell_polyfill |
| oxc_allocator | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_ast | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_ast_macros | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_ast_visit | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_data_structures | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_diagnostics | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_ecmascript | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_estree | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_index | 5.0.0 | MIT | https://github.com/oxc-project/oxc-index-vec |
| oxc_parser | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_regular_expression | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_resolver | 11.24.3 | MIT | https://github.com/oxc-project/oxc-resolver |
| oxc_span | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_str | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| oxc_syntax | 0.156.0 | MIT | https://github.com/oxc-project/oxc |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| pathdiff | 0.2.3 | MIT/Apache-2.0 | https://github.com/Manishearth/pathdiff |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ |
| phf | 0.14.0 | MIT | https://github.com/rust-phf/rust-phf |
| phf_generator | 0.14.0 | MIT | https://github.com/rust-phf/rust-phf |
| phf_macros | 0.14.0 | MIT | https://github.com/rust-phf/rust-phf |
| phf_shared | 0.14.0 | MIT | https://github.com/rust-phf/rust-phf |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project-lite |
| pnp | 0.12.12 | BSD-2-Clause | https://github.com/yarnpkg/pnp-rs |
| potential_utf | 0.1.6 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| quote | 1.0.47 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| radix_trie | 0.3.0 | MIT | https://github.com/michaelsproul/rust_radix_trie |
| rayon | 1.12.0 | MIT OR Apache-2.0 | https://github.com/rayon-rs/rayon |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 | https://github.com/rayon-rs/rayon |
| redox_syscall | 0.5.18 | MIT | https://gitlab.redox-os.org/redox-os/syscall |
| ref-cast | 1.0.27 | MIT OR Apache-2.0 | https://github.com/dtolnay/ref-cast |
| ref-cast-impl | 1.0.27 | MIT OR Apache-2.0 | https://github.com/dtolnay/ref-cast |
| regex | 1.13.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| regress | 0.12.0 | MIT OR Apache-2.0 | https://github.com/ridiculousfish/regress |
| roxmltree | 0.21.2 | MIT OR Apache-2.0 | https://github.com/RazrFalcon/roxmltree |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | https://github.com/rust-lang/rustc-hash |
| rustix | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/rustix |
| rustversion | 1.0.23 | MIT OR Apache-2.0 | https://github.com/dtolnay/rustversion |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | https://github.com/dtolnay/ryu |
| same-file | 1.0.6 | Unlicense/MIT | https://github.com/BurntSushi/same-file |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | https://github.com/bluss/scopeguard |
| self_cell | 1.3.0 | Apache-2.0 OR GPL-2.0-only | https://github.com/Voultapher/self_cell |
| seq-macro | 0.3.6 | MIT OR Apache-2.0 | https://github.com/dtolnay/seq-macro |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| simd-json | 0.18.1 | Apache-2.0 OR MIT | https://github.com/simd-lite/simd-json |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | https://github.com/rusticstuff/simdutf8 |
| siphasher | 1.0.3 | MIT/Apache-2.0 | https://github.com/jedisct1/rust-siphash |
| smallvec | 1.16.1 | MIT OR Apache-2.0 | https://github.com/servo/rust-smallvec |
| smawk | 0.3.3 | MIT | https://github.com/mgeisler/smawk |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | https://github.com/storyyeller/stable_deref_trait |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | https://github.com/nvzqz/static-assertions-rs |
| strsim | 0.11.1 | MIT | https://github.com/rapidfuzz/strsim-rs |
| syn | 2.0.119 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| syn | 3.0.5 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| synstructure | 0.13.2 | MIT | https://github.com/mystor/synstructure |
| textwrap | 0.16.4 | MIT | https://github.com/mgeisler/textwrap |
| thiserror | 2.0.20 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| thiserror-impl | 2.0.20 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| tinystr | 0.8.4 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| tracing | 0.1.44 | MIT | https://github.com/tokio-rs/tracing |
| tracing-attributes | 0.1.31 | MIT | https://github.com/tokio-rs/tracing |
| tracing-core | 0.1.36 | MIT | https://github.com/tokio-rs/tracing |
| unicode-id-start | 1.4.0 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/Boshen/unicode-id-start |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-segmentation |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-width |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | https://github.com/hsivonen/utf8_iter |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT | https://github.com/alacritty/vte |
| value-trait | 0.12.2 | Apache-2.0/MIT | https://github.com/simd-lite/value-trait |
| walkdir | 2.5.0 | Unlicense/MIT | https://github.com/BurntSushi/walkdir |
| winapi-util | 0.1.11 | Unlicense OR MIT | https://github.com/BurntSushi/winapi-util |
| windows | 0.62.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| writeable | 0.6.4 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| xxhash-rust | 0.8.19 | BSL-1.0 | https://github.com/DoumanAsh/xxhash-rust |
| yaml-rust2 | 0.13.0 | MIT OR Apache-2.0 | https://github.com/Ethiraric/yaml-rust2 |
| yoke | 0.8.3 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| yoke-derive | 0.8.2 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| zerofrom | 0.1.8 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| zerofrom-derive | 0.1.7 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| zerotrie | 0.2.5 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| zerovec | 0.11.8 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| zerovec-derive | 0.11.6 | Unicode-3.0 | https://github.com/unicode-org/icu4x |
| zlib-rs | 0.6.8 | Zlib | https://github.com/trifectatechfoundation/zlib-rs |
| zmij | 1.0.23 | MIT | https://github.com/dtolnay/zmij |

<!-- BEGIN fixture notices -->
## Corpus and fixture files

The test and fixture files below were copied or adapted from other projects. Each was
compared with the upstream commit its source link pins. The full upstream licence texts
and copyright notices follow.

| Shipped file | Source repository | Commit | Upstream path | Licence | Adaptation |
| --- | --- | --- | --- | --- | --- |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/client/api.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/client/api.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/client/platforms/anthropic.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/client/platforms/anthropic.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/client/platforms/baidu.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/client/platforms/baidu.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/client/platforms/google.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/client/platforms/google.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/client/platforms/openai.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/client/platforms/openai.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/client/platforms/tencent.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/client/platforms/tencent.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/artifacts.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/artifacts.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/chat.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/chat.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/exporter.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/exporter.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/home.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/home.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/markdown.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/markdown.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/mask.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/mask.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/mcp-market.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/mcp-market.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/message-selector.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/message-selector.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/plugin.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/plugin.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/realtime-chat/realtime-chat.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/realtime-chat/realtime-chat.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/settings.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/settings.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/sidebar.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/sidebar.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/components/ui-lib.tsx`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/components/ui-lib.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/mcp/actions.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/mcp/actions.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/store/access.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/store/access.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/store/chat.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/store/chat.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/store/plugin.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/store/plugin.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`app/utils/chat.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/app/utils/chat.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/compositions/src/examples/rich-text-editor/rich-text-editor-composition.tsx`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/compositions/src/examples/rich-text-editor/rich-text-editor-composition.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/compositions/src/ui/rich-text-editor-control.tsx`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/compositions/src/ui/rich-text-editor-control.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/compositions/src/ui/rich-text-editor-menu.tsx`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/compositions/src/ui/rich-text-editor-menu.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/www/components/component-explorer-sidebar.tsx`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/www/components/component-explorer-sidebar.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/www/scripts/generate-types.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/www/scripts/generate-types.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/www/utils/get-theming-doc.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/www/utils/get-theming-doc.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`apps/www/utils/project-sdk.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/apps/www/utils/project-sdk.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/charts/src/chart/chart.tsx`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/charts/src/chart/chart.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/charts/src/use-chart.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/charts/src/use-chart.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/cli/src/commands/blocks.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/cli/src/commands/blocks.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/components/menu.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/components/menu.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/components/popover.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/components/popover.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/components/steps.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/components/steps.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/props/as-props.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/props/as-props.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/removed/nextjs-package.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/removed/nextjs-package.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/removed/stack-divider.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/removed/stack-divider.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/transforms/theme/theme-tokens.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/transforms/theme/theme-tokens.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/upgrade.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/upgrade.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/src/utils/chakra-tracker.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/src/utils/chakra-tracker.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/react/scripts/typedocs.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/react/scripts/typedocs.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/react/src/preset-base.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/react/src/preset-base.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/react/src/styled-system/factory.tsx`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/react/src/styled-system/factory.tsx) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/react/src/styled-system/token-dictionary.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/react/src/styled-system/token-dictionary.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/m6-dev-checks.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/react/src/styled-system/utility.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/react/src/styled-system/utility.ts) | MIT | Analyzer report captured on this pinned source. Report prose is first-party; source paths and evidence refer to the upstream project. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/adapter.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/adapter.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/chat-command-prefix.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/chat-command-prefix.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/cloudflare.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/cloudflare.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/collect-model-table.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/collect-model-table.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/collect-model-with-default.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/collect-model-with-default.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/create-message.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/create-message.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/estimate-token-length.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/estimate-token-length.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/extract-mcp-json.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/extract-mcp-json.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/format-chunks.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/format-chunks.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/format.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/format.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/get-bearer-token.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/get-bearer-token.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/get-client-config.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/get-client-config.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/get-operation-id.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/get-operation-id.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/hmac-hash.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/hmac-hash.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/hmac-hex.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/hmac-hex.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/hmac-sign.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/hmac-sign.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/is-gpt4-model.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/is-gpt4-model.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/is-mcp-json.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/is-mcp-json.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/limit-number.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/limit-number.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/merge.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/merge.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/modal-config-validator.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/modal-config-validator.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/model-available-in-server.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/model-available-in-server.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/model-available.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/model-available.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/model-helpers.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/model-helpers.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/model-provider.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/model-provider.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/object-utils.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/object-utils.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/platform-detection.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/platform-detection.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/prosody-options.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/prosody-options.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/safe-local-storage.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/safe-local-storage.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/sum-module.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/sum-module.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/tts-config-validator.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/tts-config-validator.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/utils.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/utils.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/valid-string.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/valid-string.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-bench/tests/fixtures/nextchat-list-slice.json` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/vision-model-checker.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/vision-model-checker.test.ts) | MIT | `spirolo list` output captured on this pinned source. The JSON shape is first-party; test titles and paths are the upstream project's. |
| `crates/spirolo-frameworks/tests/body_hash.rs` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/react/src/components/ComboBox/ComboBox-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/react/src/components/ComboBox/ComboBox-test.js) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. One test body adapted from the restore-on-blur tests at lines 587 and 605: title shortened, clear/type steps dropped, the selected-item attribute parameterised. Copyright IBM Corp. 2016, 2026 |
| `crates/spirolo-frameworks/tests/fixtures/reset-mocks/jest.config.js` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`backend/jest.config.js`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/backend/jest.config.js) | MIT | Copied upstream file; see the fixture provenance for any cuts. |
| `crates/spirolo-inputs/tests/fixtures/valtio-vitest-4.1.5/proxy.test.ts` | [pmndrs/valtio](https://github.com/pmndrs/valtio) | `6c4af4ba642b959e86e698c495176fdce8646c05` | [`tests/vanilla/proxy.test.ts`](https://github.com/pmndrs/valtio/blob/6c4af4ba642b959e86e698c495176fdce8646c05/tests/vanilla/proxy.test.ts) | MIT | Copied upstream file; see the fixture provenance for any cuts. |
| `crates/spirolo-model/tests/fixtures/generated/clipboard-outline.ts` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`src/eva-icons/icons/clipboard-outline.ts`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/src/eva-icons/icons/clipboard-outline.ts) | MIT | Copied verbatim; upstream generated it from Eva Icons 1.1.3. Eva Icons: Copyright (c) 2018 Akveo. |
| `crates/spirolo-model/tests/fixtures/m6-polls/action-poll.ts` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/@expo/cli/src/utils/delay.ts`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/@expo/cli/src/utils/delay.ts) | MIT | Copied upstream file; see the fixture provenance for any cuts. |
| `crates/spirolo-model/tests/fixtures/m6-polls/do-poll.ts` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`apps/test-suite/tests/AppMetrics.ts`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/apps/test-suite/tests/AppMetrics.ts) | MIT | Extracted pollUntil function from the pinned upstream file; parsed only. |
| `crates/spirolo-model/tests/fixtures/m6-polls/orpc.LICENSE` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`LICENSE`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/LICENSE) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo-model/tests/fixtures/m6-polls/retry-after.ts` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`packages/client/src/plugins/retry-after.ts`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/packages/client/src/plugins/retry-after.ts) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo-model/tests/fixtures/m6-polls/while-poll.ts` | [fastrepl/anarlog](https://github.com/fastrepl/anarlog) | `3de75bfb22d876be27e15a038e3e319091b6d016` | [`apps/desktop/src/imports/connected-import.ts`](https://github.com/fastrepl/anarlog/blob/3de75bfb22d876be27e15a038e3e319091b6d016/apps/desktop/src/imports/connected-import.ts) | MIT | Extracted waitForNangoConnection function from the pinned upstream file; parsed only. |
| `crates/spirolo-model/tests/fixtures/named-reset/agentService.test.ts` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`packages/api/src/opamp/services/__tests__/agentService.test.ts`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/packages/api/src/opamp/services/__tests__/agentService.test.ts) | MIT | Copied upstream file; see the fixture provenance for any cuts. |
| `crates/spirolo-model/tests/fixtures/named-reset/useSidePanelStack.test.tsx` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`packages/app/src/hooks/__tests__/useSidePanelStack.test.tsx`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/packages/app/src/hooks/__tests__/useSidePanelStack.test.tsx) | MIT | Copied upstream file; see the fixture provenance for any cuts. |
| `crates/spirolo-rules/tests/stale_snapshot.rs` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`src/processor/js/src/service/mapping/__snapshots__/mapping.spec.js.snap`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/src/processor/js/src/service/mapping/__snapshots__/mapping.spec.js.snap) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo-rules/tests/stale_snapshot.rs` | [callstack/react-native-paper](https://github.com/callstack/react-native-paper) | `4ffc20ae4ac463cff70f50e22bc627531ec59467` | [`src/components/__tests__/RadioButton/__snapshots__/RadioButton.test.tsx.snap`](https://github.com/callstack/react-native-paper/blob/4ffc20ae4ac463cff70f50e22bc627531ec59467/src/components/__tests__/RadioButton/__snapshots__/RadioButton.test.tsx.snap) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo-rules/tests/stale_snapshot.rs` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`packages/create-instantsearch-app/e2e/templates.test.js`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/packages/create-instantsearch-app/e2e/templates.test.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo-rules/tests/stale_snapshot.rs` | [react/metro](https://github.com/react/metro) | `5793c17017825d1d59191d080114365e1da569ae` | [`packages/metro-symbolicate/src/__tests__/__snapshots__/symbolicate-test.js.snap`](https://github.com/react/metro/blob/5793c17017825d1d59191d080114365e1da569ae/packages/metro-symbolicate/src/__tests__/__snapshots__/symbolicate-test.js.snap) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo-rules/tests/stale_snapshot.rs` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/semantic-ui/test/Grid.test.tsx`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/semantic-ui/test/Grid.test.tsx) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`packages/decap-cms-widget-markdown/src/__tests__/renderer.spec.js`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/packages/decap-cms-widget-markdown/src/__tests__/renderer.spec.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`packages/instantsearch-ui-components/src/components/chat/__tests__/ChatOverlayLayout.test.tsx`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/packages/instantsearch-ui-components/src/components/chat/__tests__/ChatOverlayLayout.test.tsx) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`frontend/components/Admin/__tests__/AdminUsersPage.test.tsx`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/frontend/components/Admin/__tests__/AdminUsersPage.test.tsx) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`examples/CustomSingle.test.tsx`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/examples/CustomSingle.test.tsx) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`test/elements.ts`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/test/elements.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/@expo/cli/e2e/playwright/prod/server-loader.test.ts`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/@expo/cli/e2e/playwright/prod/server-loader.test.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/absence_anchor.rs` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/core/test/LayoutGridField.test.tsx`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/core/test/LayoutGridField.test.tsx) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/dead_skip.rs` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-ui-materialui/src/input/FileInput.spec.tsx`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-ui-materialui/src/input/FileInput.spec.tsx) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/dead_skip.rs` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-ui-materialui/src/input/ImageInput.spec.tsx`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-ui-materialui/src/input/ImageInput.spec.tsx) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/dead_skip.rs` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`integration-tests/structured-logging/__tests__/to-do.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/integration-tests/structured-logging/__tests__/to-do.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/dead_skip.rs` | [mobxjs/mobx](https://github.com/mobxjs/mobx) | `404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479` | [`packages/mobx/__tests__/base/typescript-tests.ts`](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/packages/mobx/__tests__/base/typescript-tests.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/dead_skip.rs` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`packages/react/slot/src/slot.test.tsx`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/packages/react/slot/src/slot.test.tsx) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/duplicate_test.rs` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/duplicate_test.rs` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/duplicate_test.rs` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/duplicate_test.rs` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/duplicate_test.rs` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/duplicate_test.rs` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/duplicate_test.rs` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/setupTests.js`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/setupTests.js) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`jsdom.mocks.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/jsdom.mocks.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`tests/utils/setupTests.ts`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/tests/utils/setupTests.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`jest.setup.js`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/jest.setup.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react-devtools-shared/src/__tests__/setupTests.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react-devtools-shared/src/__tests__/setupTests.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. Copyright (c) Meta Platforms, Inc. and affiliates. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`config/jest-config-carbon/setup/setupAfterEnv.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/config/jest-config-carbon/setup/setupAfterEnv.js) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. Copyright IBM Corp. 2016, 2023 |
| `crates/spirolo/tests/fail_closed_setup.rs` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`config/jest-config-carbon/index.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/config/jest-config-carbon/index.js) | Apache-2.0 | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. Copyright IBM Corp. 2018, 2023 |
| `crates/spirolo/tests/fail_closed_setup.rs` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/expo-module-scripts/jest-preset.cjs`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/expo-module-scripts/jest-preset.cjs) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/universal/jest-preset.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/universal/jest-preset.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [fuma-nama/fumadocs](https://github.com/fuma-nama/fumadocs) | `bb43b610100da6324f561ec3b4049e59f1a40663` | [`packages/create-app/vitest.config.ts`](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/packages/create-app/vitest.config.ts) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/plugin-oxc/jest.config.js`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/plugin-oxc/jest.config.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/plugin/jest.config.js`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/plugin/jest.config.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fail_closed_setup.rs` | [react-hook-form/react-hook-form](https://github.com/react-hook-form/react-hook-form) | `c4dc4456e60c20833a2c70931128224e6cb13176` | [`scripts/jest/jest.config.js`](https://github.com/react-hook-form/react-hook-form/blob/c4dc4456e60c20833a2c70931128224e6cb13176/scripts/jest/jest.config.js) | MIT | Contains adapted inline test/config excerpts; surrounding Rust harness is first-party. |
| `crates/spirolo/tests/fixtures/absence-anchor/carbon.test.jsx` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/react/src/components/AddSelect/__tests__/AddSelect-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/react/src/components/AddSelect/__tests__/AddSelect-test.js) | Apache-2.0 | Test bodies copied unchanged; imports and enclosing suites trimmed; parsed only. Copyright IBM Corp. 2026 |
| `crates/spirolo/tests/fixtures/absence-anchor/carbon.test.jsx` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/react/src/components/Dialog/Dialog-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/react/src/components/Dialog/Dialog-test.js) | Apache-2.0 | Test bodies copied unchanged; imports and enclosing suites trimmed; parsed only. Copyright IBM Corp. 2023, 2026 |
| `crates/spirolo/tests/fixtures/absence-anchor/LICENSE-carbon` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`LICENSE`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/LICENSE) | Apache-2.0 | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/absence-anchor/LICENSE-rjsf.md` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`LICENSE.md`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/LICENSE.md) | Apache-2.0 | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/absence-anchor/rjsf.test.tsx` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/core/test/LayoutGridField.test.tsx`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/core/test/LayoutGridField.test.tsx) | Apache-2.0 | Test bodies copied unchanged; imports and enclosing suites trimmed; parsed only. |
| `crates/spirolo/tests/fixtures/absence-anchor/rjsf.test.tsx` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/daisyui/test/DaisyUIForm.test.tsx`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/daisyui/test/DaisyUIForm.test.tsx) | Apache-2.0 | Test bodies copied unchanged; imports and enclosing suites trimmed; parsed only. |
| `crates/spirolo/tests/fixtures/discovery-config-root/configs/jest.config.js` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`configs/jest.config.js`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/configs/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-config-root/packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.js` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.js`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-config-root/packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.node.js` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.node.js`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.node.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-explicit-import/test/vision-model-checker.test.ts` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/vision-model-checker.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/vision-model-checker.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-generated/packages/@mantine/carousel/src/CarouselSlide/CarouselSlide.test.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/carousel/src/CarouselSlide/CarouselSlide.test.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/carousel/src/CarouselSlide/CarouselSlide.test.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-generated/packages/@mantine/charts/src/AreaChart/AreaChart.test.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/charts/src/AreaChart/AreaChart.test.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/charts/src/AreaChart/AreaChart.test.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-glob-groups/jest.config.js` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`jest.config.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-glob-groups/src/Collection/test.js` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Collection/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Collection/test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-glob-groups/src/QueryDescription/__tests__/queryWithoutDeleted.test.js` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/QueryDescription/__tests__/queryWithoutDeleted.test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/QueryDescription/__tests__/queryWithoutDeleted.test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-jest-extensions/package.json` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`package.json`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/package.json) | MIT | Root defaults scenario: package manifests and two test files copied verbatim; child config omitted, as in the root runner listing. |
| `crates/spirolo/tests/fixtures/discovery-jest-extensions/packages/react-native-worklets/package.json` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/package.json`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/package.json) | MIT | Root defaults scenario: package manifests and two test files copied verbatim; child config omitted, as in the root runner listing. |
| `crates/spirolo/tests/fixtures/discovery-jest-extensions/packages/react-native-worklets/plugin-oxc/test/babel-shim.test.mts` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/plugin-oxc/test/babel-shim.test.mts`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/plugin-oxc/test/babel-shim.test.mts) | MIT | Root defaults scenario: package manifests and two test files copied verbatim; child config omitted, as in the root runner listing. |
| `crates/spirolo/tests/fixtures/discovery-jest-extensions/packages/react-native-worklets/plugin-oxc/test/napi.test.mts` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/plugin-oxc/test/napi.test.mts`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/plugin-oxc/test/napi.test.mts) | MIT | Root defaults scenario: package manifests and two test files copied verbatim; child config omitted, as in the root runner listing. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/LICENSE` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`LICENSE`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/LICENSE) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/package.json` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`package.json`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/package.json) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/api-bridge/jest.config.js` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/api-bridge/jest.config.js`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/api-bridge/jest.config.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/api-bridge/package.json` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/api-bridge/package.json`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/api-bridge/package.json) | MIT | Upstream file with its author email line removed; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/api-bridge/test/esm-dist.test.mjs` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/api-bridge/test/esm-dist.test.mjs`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/api-bridge/test/esm-dist.test.mjs) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/web/jest.config.js` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/web/jest.config.js`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/web/jest.config.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/web/package.json` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/web/package.json`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/web/package.json) | MIT | Upstream file with its author email line removed; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/web/test/fixtures/enhance/base-app-hackernews/package.json` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/web/test/fixtures/enhance/base-app-hackernews/package.json`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/web/test/fixtures/enhance/base-app-hackernews/package.json) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/web/test/fixtures/enhance/base-app-hackernews/test/app/controller/news.test.js` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/web/test/fixtures/enhance/base-app-hackernews/test/app/controller/news.test.js`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/web/test/fixtures/enhance/base-app-hackernews/test/app/controller/news.test.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/packages/web/test/fixtures/enhance/base-app-hackernews/test/app/service/HackerNews.test.js` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`packages/web/test/fixtures/enhance/base-app-hackernews/test/app/service/HackerNews.test.js`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/packages/web/test/fixtures/enhance/base-app-hackernews/test/app/service/HackerNews.test.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/pnpm-workspace.yaml` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`pnpm-workspace.yaml`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/pnpm-workspace.yaml) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-root-defaults/runner-list.json` | [midwayjs/midway](https://github.com/midwayjs/midway) | `bd306bb74b025168b4a7386cff78aacf7862a81e` | [`package.json`](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/package.json) | MIT | Real Jest listing generated from this pinned upstream root; checkout prefix removed. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/LICENSE` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`LICENSE`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/LICENSE) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/package.json) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/packages/react-devtools-shared/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react-devtools-shared/package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react-devtools-shared/package.json) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/packages/react-devtools-shared/src/__tests__/events-test.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react-devtools-shared/src/__tests__/events-test.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react-devtools-shared/src/__tests__/events-test.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/packages/react/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react/package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react/package.json) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/packages/react/src/__tests__/ReactCoffeeScriptClass-test.coffee` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react/src/__tests__/ReactCoffeeScriptClass-test.coffee`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react/src/__tests__/ReactCoffeeScriptClass-test.coffee) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/runner-list.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/jest/jest-cli.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/jest/jest-cli.js) | MIT | Real Jest listing generated through this pinned upstream wrapper; wrapper log and checkout prefix removed. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/scripts/jest/config.base.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/jest/config.base.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/jest/config.base.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/scripts/jest/config.source.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/jest/config.source.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/jest/config.source.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/scripts/jest/dont-run-jest-directly.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/jest/dont-run-jest-directly.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/jest/dont-run-jest-directly.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/scripts/jest/jest-cli.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/jest/jest-cli.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/jest/jest-cli.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/scripts/jest/jest.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/jest/jest.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/jest/jest.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jest-wrapper/scripts/shared/__tests__/evalToString-test.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/shared/__tests__/evalToString-test.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/shared/__tests__/evalToString-test.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-jsx/packages/mui-lab/src/TimelineContent/TimelineContent.test.js` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`packages/mui-lab/src/TimelineContent/TimelineContent.test.js`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/packages/mui-lab/src/TimelineContent/TimelineContent.test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-jsx/packages/mui-lab/src/TimelineDot/TimelineDot.test.js` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`packages/mui-lab/src/TimelineDot/TimelineDot.test.js`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/packages/mui-lab/src/TimelineDot/TimelineDot.test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-jsx/vitest.config.mts` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`vitest.config.mts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/vitest.config.mts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-nextchat/jest.config.ts` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`jest.config.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/jest.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-nextchat/test/format.test.ts` | [ChatGPTNextWeb/NextChat](https://github.com/ChatGPTNextWeb/NextChat) | `defdcdb55d850cd12c4c657eb83729fd66e215c0` | [`test/format.test.ts`](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/test/format.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-package-runner/package.json` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`package.json`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-package-runner/packages/arcs/package.json` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`packages/arcs/package.json`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/arcs/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-package-runner/packages/arcs/tests/boundingBox.test.ts` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`packages/arcs/tests/boundingBox.test.ts`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/arcs/tests/boundingBox.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-playwright/e2e/toast.spec.ts` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`e2e/toast.spec.ts`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/e2e/toast.spec.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-playwright/e2e/toggle-group.spec.ts` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`e2e/toggle-group.spec.ts`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/e2e/toggle-group.spec.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-playwright/playwright.config.ts` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`playwright.config.ts`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/playwright.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-project-roots/examples/AsiaSaigonTimezone.test.tsx` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`examples/AsiaSaigonTimezone.test.tsx`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/examples/AsiaSaigonTimezone.test.tsx) | MIT | First two inline project objects retained verbatim; parent export preserved. |
| `crates/spirolo/tests/fixtures/discovery-project-roots/examples/timezone/TestCase2833.test.tsx` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`examples/timezone/TestCase2833.test.tsx`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/examples/timezone/TestCase2833.test.tsx) | MIT | First two inline project objects retained verbatim; parent export preserved. |
| `crates/spirolo/tests/fixtures/discovery-project-roots/jest.config.ts` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`jest.config.ts`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/jest.config.ts) | MIT | First two inline project objects retained verbatim; parent export preserved. |
| `crates/spirolo/tests/fixtures/discovery-projects/packages/core/src/components/Spring.test-d.tsx` | [pmndrs/react-spring](https://github.com/pmndrs/react-spring) | `d9cd058a07c7ac5b576f40a5f4312f4471897661` | [`packages/core/src/components/Spring.test-d.tsx`](https://github.com/pmndrs/react-spring/blob/d9cd058a07c7ac5b576f40a5f4312f4471897661/packages/core/src/components/Spring.test-d.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-projects/packages/core/src/helpers.test.ts` | [pmndrs/react-spring](https://github.com/pmndrs/react-spring) | `d9cd058a07c7ac5b576f40a5f4312f4471897661` | [`packages/core/src/helpers.test.ts`](https://github.com/pmndrs/react-spring/blob/d9cd058a07c7ac5b576f40a5f4312f4471897661/packages/core/src/helpers.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-projects/vitest.config.ts` | [pmndrs/react-spring](https://github.com/pmndrs/react-spring) | `d9cd058a07c7ac5b576f40a5f4312f4471897661` | [`vitest.config.ts`](https://github.com/pmndrs/react-spring/blob/d9cd058a07c7ac5b576f40a5f4312f4471897661/vitest.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-regex/packages/metro-babel-register/src/__tests__/parses-without-transpilation-test.js` | [react/metro](https://github.com/react/metro) | `5793c17017825d1d59191d080114365e1da569ae` | [`packages/metro-babel-register/src/__tests__/parses-without-transpilation-test.js`](https://github.com/react/metro/blob/5793c17017825d1d59191d080114365e1da569ae/packages/metro-babel-register/src/__tests__/parses-without-transpilation-test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-regex/packages/metro-resolver/src/__tests__/utils.js` | [react/metro](https://github.com/react/metro) | `5793c17017825d1d59191d080114365e1da569ae` | [`packages/metro-resolver/src/__tests__/utils.js`](https://github.com/react/metro/blob/5793c17017825d1d59191d080114365e1da569ae/packages/metro-resolver/src/__tests__/utils.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-root-ignore/jest.config.js` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`jest.config.js`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/jest.config.js) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-root-ignore/LICENSE.txt` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`LICENSE.txt`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/LICENSE.txt) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-root-ignore/runner-list.json` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`jest.config.js`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/jest.config.js) | MIT | Real Jest listing generated from this pinned upstream config; checkout prefix removed. |
| `crates/spirolo/tests/fixtures/discovery-root-ignore/src/codemod/src/__tests__/docs.spec.ts` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`src/codemod/src/__tests__/docs.spec.ts`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/src/codemod/src/__tests__/docs.spec.ts) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/discovery-rootdir/__tests__/conf/themeSwitch.xuhome.test.js` | [notionnext-org/NotionNext](https://github.com/notionnext-org/NotionNext) | `50e77e069ec48ebd3d641888c13cd5ec6842c3d2` | [`__tests__/conf/themeSwitch.xuhome.test.js`](https://github.com/notionnext-org/NotionNext/blob/50e77e069ec48ebd3d641888c13cd5ec6842c3d2/__tests__/conf/themeSwitch.xuhome.test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-rootdir/__tests__/lib/cache/cache_key_path.test.js` | [notionnext-org/NotionNext](https://github.com/notionnext-org/NotionNext) | `50e77e069ec48ebd3d641888c13cd5ec6842c3d2` | [`__tests__/lib/cache/cache_key_path.test.js`](https://github.com/notionnext-org/NotionNext/blob/50e77e069ec48ebd3d641888c13cd5ec6842c3d2/__tests__/lib/cache/cache_key_path.test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-rootdir/jest.config.js` | [notionnext-org/NotionNext](https://github.com/notionnext-org/NotionNext) | `50e77e069ec48ebd3d641888c13cd5ec6842c3d2` | [`jest.config.js`](https://github.com/notionnext-org/NotionNext/blob/50e77e069ec48ebd3d641888c13cd5ec6842c3d2/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-workspace-reach/apps/api/package.json` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`apps/api/package.json`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/apps/api/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-workspace-reach/apps/api/vitest.config.ts` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`apps/api/vitest.config.ts`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/apps/api/vitest.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-workspace-reach/pnpm-workspace.yaml` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`pnpm-workspace.yaml`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/pnpm-workspace.yaml) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-workspace-reach/tests/api/column/to-slug.test.ts` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`tests/api/column/to-slug.test.ts`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/tests/api/column/to-slug.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/discovery-workspace-reach/tests/api/mcp-internal-api-url.test.ts` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`tests/api/mcp-internal-api-url.test.ts`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/tests/api/mcp-internal-api-url.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/as-props.test.ts` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/as-props.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/as-props.test.ts) | MIT | Trimmed to the recorded test at line 507 and its enclosing suites; kept lines unchanged; parsed only. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/LICENSE` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`LICENSE`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/LICENSE) | MIT | Unmodified upstream file; the adjacent README pins the source. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/run-01-idle/report-packages__codemod.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/as-props.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/as-props.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/run-01-idle/report-root.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/as-props.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/as-props.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/run-02-load/report-packages__codemod.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/as-props.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/as-props.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/run-02-load/report-root.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/as-props.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/as-props.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/true/cross-file.test.ts` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/cross-file.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/cross-file.test.ts) | MIT | Trimmed to the recorded test at line 7 and its enclosing suite; kept lines unchanged; parsed only. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/true/run-01-idle/report-packages__codemod.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/cross-file.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/cross-file.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/true/run-01-idle/report-root.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/cross-file.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/cross-file.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/true/run-10-load/report-packages__codemod.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/cross-file.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/cross-file.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/flake-observed-chakra-homes/true/run-10-load/report-root.json` | [chakra-ui/chakra-ui](https://github.com/chakra-ui/chakra-ui) | `961161428b8c59157ad921dd23303b73c294d73f` | [`packages/codemod/__tests__/cross-file.test.ts`](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/packages/codemod/__tests__/cross-file.test.ts) | MIT | Vitest JSON report from a corpus run on this pinned source, cut to the recorded file result; the report shape is the runner's, test titles and paths are the upstream project's, the checkout root is neutral. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/carbon-playwright/.github/workflows/ci.yml` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`.github/workflows/ci.yml`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/.github/workflows/ci.yml) | Apache-2.0 | Workflow cut to its avt-runner job. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/carbon-playwright/package.json` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`package.json`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/package.json) | Apache-2.0 | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/carbon-playwright/packages/web-components/playwright.config.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/playwright.config.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/playwright.config.js) | Apache-2.0 | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/carbon-playwright/playwright.config.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`playwright.config.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/playwright.config.js) | Apache-2.0 | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/extension-playwright/.github/workflows/e2e.yml` | [extension-js/extension.js](https://github.com/extension-js/extension.js) | `8143b84d990a50225a94ea6532acfa1e32c423e1` | [`.github/workflows/e2e.yml`](https://github.com/extension-js/extension.js/blob/8143b84d990a50225a94ea6532acfa1e32c423e1/.github/workflows/e2e.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/extension-playwright/package.json` | [extension-js/extension.js](https://github.com/extension-js/extension.js) | `8143b84d990a50225a94ea6532acfa1e32c423e1` | [`package.json`](https://github.com/extension-js/extension.js/blob/8143b84d990a50225a94ea6532acfa1e32c423e1/package.json) | MIT | Package manifest cut to its name, private flag and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/extension-playwright/playwright.config.ts` | [extension-js/extension.js](https://github.com/extension-js/extension.js) | `8143b84d990a50225a94ea6532acfa1e32c423e1` | [`playwright.config.ts`](https://github.com/extension-js/extension.js/blob/8143b84d990a50225a94ea6532acfa1e32c423e1/playwright.config.ts) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/extension-playwright/scripts/run-e2e.mjs` | [extension-js/extension.js](https://github.com/extension-js/extension.js) | `8143b84d990a50225a94ea6532acfa1e32c423e1` | [`scripts/run-e2e.mjs`](https://github.com/extension-js/extension.js/blob/8143b84d990a50225a94ea6532acfa1e32c423e1/scripts/run-e2e.mjs) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/gatsby-circleci/.circleci/config.yml` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`.circleci/config.yml`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/.circleci/config.yml) | MIT | CircleCI config cut to its aliases, the unit_tests_node18 job and its build-test workflow entry. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/gatsby-circleci/.github/workflows/schedule-stale.yml` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`.github/workflows/schedule-stale.yml`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/.github/workflows/schedule-stale.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/gatsby-circleci/package.json` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`package.json`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/package.json) | MIT | Package manifest cut to its scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/instantsearch-playwright/.github/workflows/e2e.yml` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`.github/workflows/e2e.yml`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/.github/workflows/e2e.yml) | MIT | Workflow cut to its e2e-tests-playwright job. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/instantsearch-playwright/package.json` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`package.json`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/package.json) | MIT | Package manifest cut to its name, scripts and workspaces. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/instantsearch-playwright/tests/e2e/package.json` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`tests/e2e/package.json`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/tests/e2e/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/instantsearch-playwright/tests/e2e/playwright/playwright.config.ts` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`tests/e2e/playwright/playwright.config.ts`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/tests/e2e/playwright/playwright.config.ts) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/kaneo-playwright/.github/workflows/browser-tests.yml` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`.github/workflows/browser-tests.yml`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/.github/workflows/browser-tests.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/kaneo-playwright/package.json` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`package.json`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/kaneo-playwright/tests/e2e/package.json` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`tests/e2e/package.json`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/tests/e2e/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/kaneo-playwright/tests/e2e/playwright.config.ts` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`tests/e2e/playwright.config.ts`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/tests/e2e/playwright.config.ts) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/algolia__instantsearch.txt` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`LICENSE`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/carbon-design-system__carbon.txt` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`LICENSE`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/LICENSE) | Apache-2.0 | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/chrisvel__tududi.txt` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`LICENSE`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/extension-js__extension.js.txt` | [extension-js/extension.js](https://github.com/extension-js/extension.js) | `8143b84d990a50225a94ea6532acfa1e32c423e1` | [`LICENSE`](https://github.com/extension-js/extension.js/blob/8143b84d990a50225a94ea6532acfa1e32c423e1/LICENSE) | MIT | Upstream licence with the author's contact email omitted; name, year and URL kept. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/gatsbyjs__gatsby.txt` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`LICENSE`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/necolas__react-native-web.txt` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`LICENSE`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/plouc__nivo.txt` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`LICENSE.md`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/LICENSE.md) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/radix-ui__primitives.txt` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`LICENSE`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/react__react.txt` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`LICENSE`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/licenses/usekaneo__kaneo.txt` | [usekaneo/kaneo](https://github.com/usekaneo/kaneo) | `afe97be2473e0cd97fe4a63e526fef9ac4df85b0` | [`LICENSE`](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/LICENSE) | MIT | Unmodified upstream licence; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/nivo-cypress-install/.github/workflows/ci.yml` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`.github/workflows/ci.yml`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/.github/workflows/ci.yml) | MIT | Workflow cut to its end_to_end job, ending before the Cypress run step. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/nivo-cypress/.github/workflows/ci.yml` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`.github/workflows/ci.yml`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/.github/workflows/ci.yml) | MIT | Workflow cut to its end_to_end job. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/nivo-make/.github/workflows/ci.yml` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`.github/workflows/ci.yml`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/.github/workflows/ci.yml) | MIT | Workflow cut to its test job. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/nivo-make/Makefile` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`Makefile`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/Makefile) | MIT | Makefile cut to its pkgs-test target. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/nivo-make/package.json` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`package.json`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/radix-playwright/.github/workflows/build.yml` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`.github/workflows/build.yml`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/.github/workflows/build.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/radix-playwright/package.json` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`package.json`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/radix-playwright/playwright.config.ts` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`playwright.config.ts`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/playwright.config.ts) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-devtools/.github/workflows/devtools_regression_tests.yml` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`.github/workflows/devtools_regression_tests.yml`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/.github/workflows/devtools_regression_tests.yml) | MIT | Workflow cut to its run_devtools_e2e_tests_for_versions job. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-devtools/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/package.json) | MIT | Package manifest cut to its scripts and workspaces. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-devtools/packages/react-devtools-inline/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react-devtools-inline/package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react-devtools-inline/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-devtools/packages/react-devtools-inline/playwright.config.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react-devtools-inline/playwright.config.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react-devtools-inline/playwright.config.js) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-devtools/scripts/ci/run_devtools_e2e_tests.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`scripts/ci/run_devtools_e2e_tests.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/scripts/ci/run_devtools_e2e_tests.js) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-native-web-lint/.github/workflows/tests.yml` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`.github/workflows/tests.yml`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/.github/workflows/tests.yml) | MIT | Workflow cut to its format, type-check and lint jobs. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-native-web-lint/package.json` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`package.json`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-native-web/.github/workflows/tests.yml` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`.github/workflows/tests.yml`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/.github/workflows/tests.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-native-web/package.json` | [necolas/react-native-web](https://github.com/necolas/react-native-web) | `f1b53224b242d9d8dad4b647e0b6f0395a0533ae` | [`package.json`](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-playground/.github/workflows/compiler_playground.yml` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`.github/workflows/compiler_playground.yml`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/.github/workflows/compiler_playground.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-playground/compiler/apps/playground/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`compiler/apps/playground/package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/compiler/apps/playground/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-playground/compiler/apps/playground/playwright.config.js` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`compiler/apps/playground/playwright.config.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/compiler/apps/playground/playwright.config.js) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/react-playground/package.json` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`package.json`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/package.json) | MIT | Package manifest cut to its scripts and workspaces. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/tududi-local-playwright/.github/workflows/ci.yml` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`.github/workflows/ci.yml`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/.github/workflows/ci.yml) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/tududi-local-playwright/e2e/playwright.config.ts` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`e2e/playwright.config.ts`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/e2e/playwright.config.ts) | MIT | Unmodified upstream file; the fixture README pins the source. |
| `crates/spirolo/tests/fixtures/gate-truth-reliability/tududi-local-playwright/package.json` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`package.json`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/package.json) | MIT | Package manifest cut to its name and scripts. The fixture README pins the source. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/algolia__instantsearch/packages/react-instantsearch-core/src/lib/__tests__/useSearchResults.test.tsx` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`packages/react-instantsearch-core/src/lib/__tests__/useSearchResults.test.tsx`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/packages/react-instantsearch-core/src/lib/__tests__/useSearchResults.test.tsx) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/boardgameio__boardgame.io/src/client/react-native.test.js` | [boardgameio/boardgame.io](https://github.com/boardgameio/boardgame.io) | `5e9a2c94bde803fae8b081958c406c4d0a7be8ae` | [`src/client/react-native.test.js`](https://github.com/boardgameio/boardgame.io/blob/5e9a2c94bde803fae8b081958c406c4d0a7be8ae/src/client/react-native.test.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/decaporg__decap-cms/packages/decap-cms-backend-gitlab/src/__tests__/gitlab.spec.js` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`packages/decap-cms-backend-gitlab/src/__tests__/gitlab.spec.js`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/packages/decap-cms-backend-gitlab/src/__tests__/gitlab.spec.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/decaporg__decap-cms/packages/decap-cms-widget-number/src/__tests__/number.spec.js` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`packages/decap-cms-widget-number/src/__tests__/number.spec.js`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/packages/decap-cms-widget-number/src/__tests__/number.spec.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/fuma-nama__fumadocs/packages/mdx/test/macro.test.ts` | [fuma-nama/fumadocs](https://github.com/fuma-nama/fumadocs) | `bb43b610100da6324f561ec3b4049e59f1a40663` | [`packages/mdx/test/macro.test.ts`](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/packages/mdx/test/macro.test.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/fuma-nama__fumadocs/packages/notion/test/source.test.ts` | [fuma-nama/fumadocs](https://github.com/fuma-nama/fumadocs) | `bb43b610100da6324f561ec3b4049e59f1a40663` | [`packages/notion/test/source.test.ts`](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/packages/notion/test/source.test.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/gatsbyjs__gatsby/integration-tests/artifacts/__tests__/index.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`integration-tests/artifacts/__tests__/index.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/integration-tests/artifacts/__tests__/index.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/gatsbyjs__gatsby/integration-tests/artifacts/jest.config.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`integration-tests/artifacts/jest.config.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/integration-tests/artifacts/jest.config.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/gatsbyjs__gatsby/jest.config.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`jest.config.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/jest.config.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/gatsbyjs__gatsby/packages/gatsby-source-lever/src/__tests__/normalize.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`packages/gatsby-source-lever/src/__tests__/normalize.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/packages/gatsby-source-lever/src/__tests__/normalize.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/middleapi__orpc/packages/cloudflare/src/lock.test.ts` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`packages/cloudflare/src/lock.test.ts`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/packages/cloudflare/src/lock.test.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/mobxjs__mobx/jest.base.config.js` | [mobxjs/mobx](https://github.com/mobxjs/mobx) | `404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479` | [`jest.base.config.js`](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/jest.base.config.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/mobxjs__mobx/packages/mobx/__tests__/base/array.js` | [mobxjs/mobx](https://github.com/mobxjs/mobx) | `404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479` | [`packages/mobx/__tests__/base/array.js`](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/packages/mobx/__tests__/base/array.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/mobxjs__mobx/packages/mobx/jest.config.js` | [mobxjs/mobx](https://github.com/mobxjs/mobx) | `404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479` | [`packages/mobx/jest.config.js`](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/packages/mobx/jest.config.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/mobxjs__mobx/packages/mobx/jest.setup.ts` | [mobxjs/mobx](https://github.com/mobxjs/mobx) | `404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479` | [`packages/mobx/jest.setup.ts`](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/packages/mobx/jest.setup.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/mui__material-ui/test/e2e-website/material-docs.spec.ts` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-docs.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-docs.spec.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/notionnext-org__NotionNext/__tests__/lib/utils/loadExternalResource.test.js` | [notionnext-org/NotionNext](https://github.com/notionnext-org/NotionNext) | `50e77e069ec48ebd3d641888c13cd5ec6842c3d2` | [`__tests__/lib/utils/loadExternalResource.test.js`](https://github.com/notionnext-org/NotionNext/blob/50e77e069ec48ebd3d641888c13cd5ec6842c3d2/__tests__/lib/utils/loadExternalResource.test.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-flake-narrowed/Nozbe__WatermelonDB/src/sync/impl/__tests__/synchronize.test.js` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/sync/impl/__tests__/synchronize.test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/sync/impl/__tests__/synchronize.test.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/chrisvel__tududi/backend/tests/integration/subtasks-completion.test.js` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`backend/tests/integration/subtasks-completion.test.js`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/backend/tests/integration/subtasks-completion.test.js) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/fuma-nama__fumadocs/packages/image-size/test/index.test.ts` | [fuma-nama/fumadocs](https://github.com/fuma-nama/fumadocs) | `bb43b610100da6324f561ec3b4049e59f1a40663` | [`packages/image-size/test/index.test.ts`](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/packages/image-size/test/index.test.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/gpbl__react-day-picker/examples/InvalidMonthDefaultLimit.test.tsx` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`examples/InvalidMonthDefaultLimit.test.tsx`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/examples/InvalidMonthDefaultLimit.test.tsx) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/grommet__grommet/package.json` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`package.json`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/package.json) | Apache-2.0 | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/grommet__grommet/src/js/components/DateTimeInput/__tests__/DateTimeInput-test.tsx` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/DateTimeInput/__tests__/DateTimeInput-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/DateTimeInput/__tests__/DateTimeInput-test.tsx) | Apache-2.0 | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/grommet__grommet/src/setupTests.js` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/setupTests.js`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/setupTests.js) | Apache-2.0 | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/middleapi__orpc/packages/ratelimit/src/handler-plugin.test.ts` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`packages/ratelimit/src/handler-plugin.test.ts`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/packages/ratelimit/src/handler-plugin.test.ts) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/m6-wall-clock/react-hook-form__react-hook-form/src/__tests__/performance.test.tsx` | [react-hook-form/react-hook-form](https://github.com/react-hook-form/react-hook-form) | `c4dc4456e60c20833a2c70931128224e6cb13176` | [`src/__tests__/performance.test.tsx`](https://github.com/react-hook-form/react-hook-form/blob/c4dc4456e60c20833a2c70931128224e6cb13176/src/__tests__/performance.test.tsx) | MIT | Copied verbatim from the corpus manifest pin. |
| `crates/spirolo/tests/fixtures/orphan-real-boardgame/examples/react-web/package.json` | [boardgameio/boardgame.io](https://github.com/boardgameio/boardgame.io) | `5e9a2c94bde803fae8b081958c406c4d0a7be8ae` | [`examples/react-web/package.json`](https://github.com/boardgameio/boardgame.io/blob/5e9a2c94bde803fae8b081958c406c4d0a7be8ae/examples/react-web/package.json) | MIT | package.json cut to its name, its test scripts and the jest preset and testPathIgnorePatterns; the other files verbatim. |
| `crates/spirolo/tests/fixtures/orphan-real-boardgame/examples/react-web/src/app.test.js` | [boardgameio/boardgame.io](https://github.com/boardgameio/boardgame.io) | `5e9a2c94bde803fae8b081958c406c4d0a7be8ae` | [`examples/react-web/src/app.test.js`](https://github.com/boardgameio/boardgame.io/blob/5e9a2c94bde803fae8b081958c406c4d0a7be8ae/examples/react-web/src/app.test.js) | MIT | package.json cut to its name, its test scripts and the jest preset and testPathIgnorePatterns; the other files verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-build-main/jest.config.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`jest.config.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/jest.config.js) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-build-main/packages/react/__tests__/ssr-test.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/react/__tests__/ssr-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/react/__tests__/ssr-test.js) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-build-main/packages/react/package.json` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/react/package.json`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/react/package.json) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-build-out-dir/jest.config.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`jest.config.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-build-out-dir/packages/gatsby-source-wordpress/__tests__/process-node.test.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`packages/gatsby-source-wordpress/__tests__/process-node.test.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/packages/gatsby-source-wordpress/__tests__/process-node.test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-component/cypress/cypress.config.ts` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`cypress/cypress.config.ts`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/cypress/cypress.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-component/cypress/src/components/boxplot/BoxPlot.cy.tsx` | [plouc/nivo](https://github.com/plouc/nivo) | `0dae2c32052a573f9f1d66ec1b453b572119b265` | [`cypress/src/components/boxplot/BoxPlot.cy.tsx`](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/cypress/src/components/boxplot/BoxPlot.cy.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-decap/cypress.config.ts` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`cypress.config.ts`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/cypress.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-decap/cypress/e2e/markdown_widget_link_spec.js` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`cypress/e2e/markdown_widget_link_spec.js`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/cypress/e2e/markdown_widget_link_spec.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-decap/jest.config.js` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`jest.config.js`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-gatsby/e2e-tests/development-runtime/cypress.config.ts` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`e2e-tests/development-runtime/cypress.config.ts`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/e2e-tests/development-runtime/cypress.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-gatsby/e2e-tests/development-runtime/cypress/integration/graphql-typegen/updates.js` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`e2e-tests/development-runtime/cypress/integration/graphql-typegen/updates.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/e2e-tests/development-runtime/cypress/integration/graphql-typegen/updates.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-subdir/cypress/cypress.config.js` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`cypress/cypress.config.js`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/cypress/cypress.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-subdir/cypress/e2e/auth.cy.js` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`cypress/e2e/auth.cy.js`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/cypress/e2e/auth.cy.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-cypress-subdir/jest.config.js` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`jest.config.js`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-outside-workspace/docs/jest.config.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`docs/jest.config.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/docs/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-outside-workspace/docs/package.json` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`docs/package.json`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/docs/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-outside-workspace/docs/ui/components/AppConfigSchemaTable/AppConfigSchemaTable.test.tsx` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`docs/ui/components/AppConfigSchemaTable/AppConfigSchemaTable.test.tsx`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/docs/ui/components/AppConfigSchemaTable/AppConfigSchemaTable.test.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-outside-workspace/package.json` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`package.json`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-outside-workspace/pnpm-workspace.yaml` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`pnpm-workspace.yaml`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/pnpm-workspace.yaml) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-script-config/package.json` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`package.json`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-script-config/packages/api/jest.config.js` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`packages/api/jest.config.js`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/packages/api/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-script-config/packages/api/jest.int.config.js` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`packages/api/jest.int.config.js`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/packages/api/jest.int.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-script-config/packages/api/package.json` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`packages/api/package.json`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/packages/api/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-jest-script-config/packages/api/src/controllers/__tests__/timeseriesEngine.test.ts` | [hyperdxio/hyperdx](https://github.com/hyperdxio/hyperdx) | `19182e7e21d0dd987bee43f5a623acf4a7785e30` | [`packages/api/src/controllers/__tests__/timeseriesEngine.test.ts`](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/packages/api/src/controllers/__tests__/timeseriesEngine.test.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/axe.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/axe.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/axe.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/create-context-container.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/create-context-container.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/create-context-container.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/index.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/index.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/index.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/external-input-fill.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/external-input-fill.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/external-input-fill.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/inputs-test-props.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/inputs-test-props.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/inputs-test-props.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-connects-label-and-input.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-connects-label-and-input.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-connects-label-and-input.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-handles-switch-checkbox-state.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-handles-switch-checkbox-state.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-handles-switch-checkbox-state.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-asterisk.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-asterisk.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-asterisk.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-container.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-container.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-container.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-sections.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-sections.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-sections.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-elements.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-elements.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-elements.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-order.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-order.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-order.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-input-wrapper-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/inputs/it-supports-shared-input-defaults.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/inputs/it-supports-shared-input-defaults.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/inputs/it-supports-shared-input-defaults.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/it-supports-system-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/it-supports-system-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/it-supports-system-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/patch-console-error.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/patch-console-error.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/patch-console-error.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/queries/index.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/queries/index.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/queries/index.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/queries/input-wrapper.queries.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/queries/input-wrapper.queries.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/queries/input-wrapper.queries.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/render.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/render.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/render.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-has-classes.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-has-classes.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-has-classes.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-has-extend.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-has-extend.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-has-extend.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-has-static-vars-resolver.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-has-static-vars-resolver.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-has-static-vars-resolver.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-has-withProps.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-has-withProps.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-has-withProps.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-is-polymorphic.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-is-polymorphic.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-is-polymorphic.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-renders-children.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-renders-children.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-renders-children.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-classname.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-classname.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-classname.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-focus-events.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-focus-events.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-focus-events.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-hidden-visible.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-hidden-visible.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-hidden-visible.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-id.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-id.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-id.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-light-dark-hidden.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-light-dark-hidden.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-light-dark-hidden.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-mod.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-mod.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-mod.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-others.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-others.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-others.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-provider-default-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-provider-default-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-provider-default-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-ref.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-ref.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-ref.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-size.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-size.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-size.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-style-api.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-style-api.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-style-api.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-style.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-style.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-style.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-supports-variant.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-supports-variant.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-supports-variant.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/shared/it-throws-context-error.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/shared/it-throws-context-error.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/shared/it-throws-context-error.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-background-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-background-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-background-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-colors-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-colors-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-colors-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-fonts-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-fonts-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-fonts-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-margins-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-margins-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-margins-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-paddings-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-paddings-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-paddings-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-position-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-position-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-position-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/style-props/it-supports-size-props.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/style-props/it-supports-size-props.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/style-props/it-supports-size-props.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/src/wait.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/src/wait.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/src/wait.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine-tests/core/tsconfig.json` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine-tests/core/tsconfig.json`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine-tests/core/tsconfig.json) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine/schedule/src/components/YearView/YearView.test.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/schedule/src/components/YearView/YearView.test.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/schedule/src/components/YearView/YearView.test.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine/schedule/src/components/YearView/YearView.tsx` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/schedule/src/components/YearView/YearView.tsx`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/schedule/src/components/YearView/YearView.tsx) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine/schedule/src/types.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/schedule/src/types.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/schedule/src/types.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine/schedule/src/utils/index.ts` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/schedule/src/utils/index.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/schedule/src/utils/index.ts) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/packages/@mantine/schedule/tsconfig.json` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`packages/@mantine/schedule/tsconfig.json`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/packages/@mantine/schedule/tsconfig.json) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-module-name-mapper/tsconfig.json` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`tsconfig.json`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/tsconfig.json) | MIT | Files copied verbatim: the root package.json, jest.config.ts and tsconfig.json, packages/@mantine-tests/core (package.json, tsconfig.json, all of src/), and from packages/@mantine/schedule its package.json, tsconfig.json, YearView.test.tsx and the three modules that test imports relatively (src/types.ts, src/utils/index.ts, YearView.tsx). |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/package.json` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/package.json`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/__tests__/jest-test.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/__tests__/jest-test.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/__tests__/jest-test.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/default-extension.android.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/default-extension.android.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/default-extension.android.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/default-extension.ios.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/default-extension.ios.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/default-extension.ios.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/default-extension.web.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/default-extension.web.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/default-extension.web.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/fallback-extension.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/fallback-extension.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/fallback-extension.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/fallback-extension.native.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/fallback-extension.native.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/fallback-extension.native.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-platform-extension/packages/jest-expo/tests/fallback-extension.node.js` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/tests/fallback-extension.node.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/tests/fallback-extension.node.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-playwright-regex/e2e/components/FluidComboBox/FluidComboBox-test.avt.e2e.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`e2e/components/FluidComboBox/FluidComboBox-test.avt.e2e.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/e2e/components/FluidComboBox/FluidComboBox-test.avt.e2e.js) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-playwright-regex/jest.config.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`jest.config.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/jest.config.js) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-playwright-regex/playwright.config.js` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`playwright.config.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/playwright.config.js) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-reporter/test/regressions/a11y/a11yReporter.ts` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/regressions/a11y/a11yReporter.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/regressions/a11y/a11yReporter.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-reporter/test/regressions/vitest.config.ts` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/regressions/vitest.config.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/regressions/vitest.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-reporter/vitest.config.mts` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`vitest.config.mts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/vitest.config.mts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-shared-module/package.json` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`package.json`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/package.json) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-shared-module/packages/utils/test/schema.test.ts` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/utils/test/schema.test.ts`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/utils/test/schema.test.ts) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-shared-module/packages/utils/test/schema/findSelectedOptionInXxxOfTest.ts` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/utils/test/schema/findSelectedOptionInXxxOfTest.ts`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/utils/test/schema/findSelectedOptionInXxxOfTest.ts) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-shared-module/packages/utils/test/schema/index.ts` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/utils/test/schema/index.ts`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/utils/test/schema/index.ts) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-shared-module/packages/utils/vitest.config.ts` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/utils/vitest.config.ts`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/utils/vitest.config.ts) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-shared-module/testing/vitest.base.ts` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`testing/vitest.base.ts`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/testing/vitest.base.ts) | Apache-2.0 | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-testfixtures/jest.config.js` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`jest.config.js`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/jest.config.js) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-testfixtures/packages/ra-core/codemods/__testfixtures__/replace-Admin-history-useEditController.input.tsx` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-core/codemods/__testfixtures__/replace-Admin-history-useEditController.input.tsx`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-core/codemods/__testfixtures__/replace-Admin-history-useEditController.input.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-testfixtures/packages/ra-core/codemods/__testfixtures__/replace-Admin-history-useEditController.output.tsx` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-core/codemods/__testfixtures__/replace-Admin-history-useEditController.output.tsx`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-core/codemods/__testfixtures__/replace-Admin-history-useEditController.output.tsx) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-testfixtures/packages/ra-core/codemods/__tests__/replace-Admin-history.spec.ts` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-core/codemods/__tests__/replace-Admin-history.spec.ts`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-core/codemods/__tests__/replace-Admin-history.spec.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-testfixtures/packages/ra-core/codemods/replace-Admin-history.ts` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-core/codemods/replace-Admin-history.ts`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-core/codemods/replace-Admin-history.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-tsc/package.json` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`package.json`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-tsc/packages/server/package.json` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`packages/server/package.json`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/packages/server/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-tsc/packages/server/src/procedure-decorated.test-d.ts` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`packages/server/src/procedure-decorated.test-d.ts`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/packages/server/src/procedure-decorated.test-d.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-tsc/packages/server/tsconfig.json` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`packages/server/tsconfig.json`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/packages/server/tsconfig.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-tsc/pnpm-workspace.yaml` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`pnpm-workspace.yaml`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/pnpm-workspace.yaml) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-tsc/tsconfig.json` | [middleapi/orpc](https://github.com/middleapi/orpc) | `63b20537dbb6ee91a4d68c6d81fa1f53ab114964` | [`tsconfig.json`](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/tsconfig.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-vitest/package.json` | [shuding/nextra](https://github.com/shuding/nextra) | `d6e80e1dd627b781429a6ee989b15ebba688c8ea` | [`package.json`](https://github.com/shuding/nextra/blob/d6e80e1dd627b781429a6ee989b15ebba688c8ea/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-vitest/packages/nextra/package.json` | [shuding/nextra](https://github.com/shuding/nextra) | `d6e80e1dd627b781429a6ee989b15ebba688c8ea` | [`packages/nextra/package.json`](https://github.com/shuding/nextra/blob/d6e80e1dd627b781429a6ee989b15ebba688c8ea/packages/nextra/package.json) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-vitest/packages/nextra/src/server/__tests__/nextra-config.test-d.ts` | [shuding/nextra](https://github.com/shuding/nextra) | `d6e80e1dd627b781429a6ee989b15ebba688c8ea` | [`packages/nextra/src/server/__tests__/nextra-config.test-d.ts`](https://github.com/shuding/nextra/blob/d6e80e1dd627b781429a6ee989b15ebba688c8ea/packages/nextra/src/server/__tests__/nextra-config.test-d.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-vitest/packages/nextra/vitest.config.ts` | [shuding/nextra](https://github.com/shuding/nextra) | `d6e80e1dd627b781429a6ee989b15ebba688c8ea` | [`packages/nextra/vitest.config.ts`](https://github.com/shuding/nextra/blob/d6e80e1dd627b781429a6ee989b15ebba688c8ea/packages/nextra/vitest.config.ts) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/orphan-shape-typecheck-vitest/pnpm-workspace.yaml` | [shuding/nextra](https://github.com/shuding/nextra) | `d6e80e1dd627b781429a6ee989b15ebba688c8ea` | [`pnpm-workspace.yaml`](https://github.com/shuding/nextra/blob/d6e80e1dd627b781429a6ee989b15ebba688c8ea/pnpm-workspace.yaml) | MIT | Files copied verbatim. |
| `crates/spirolo/tests/fixtures/resolution-astro-site/installed/astro/package.json` | [withastro/astro](https://github.com/withastro/astro) | `1e4916333554cf3798d20cd33f4dbf91e2844891` | [`packages/astro/package.json`](https://github.com/withastro/astro/blob/1e4916333554cf3798d20cd33f4dbf91e2844891/packages/astro/package.json) | MIT | Astro 6.3.8; package.json is cut to resolution metadata, tsconfigs are verbatim. |
| `crates/spirolo/tests/fixtures/resolution-astro-site/installed/astro/tsconfigs/base.json` | [withastro/astro](https://github.com/withastro/astro) | `1e4916333554cf3798d20cd33f4dbf91e2844891` | [`packages/astro/tsconfigs/base.json`](https://github.com/withastro/astro/blob/1e4916333554cf3798d20cd33f4dbf91e2844891/packages/astro/tsconfigs/base.json) | MIT | Astro 6.3.8; package.json is cut to resolution metadata, tsconfigs are verbatim. |
| `crates/spirolo/tests/fixtures/resolution-astro-site/installed/astro/tsconfigs/strict.json` | [withastro/astro](https://github.com/withastro/astro) | `1e4916333554cf3798d20cd33f4dbf91e2844891` | [`packages/astro/tsconfigs/strict.json`](https://github.com/withastro/astro/blob/1e4916333554cf3798d20cd33f4dbf91e2844891/packages/astro/tsconfigs/strict.json) | MIT | Astro 6.3.8; package.json is cut to resolution metadata, tsconfigs are verbatim. |
| `crates/spirolo/tests/fixtures/resolution-astro-site/project/tsconfig.json` | [withastro/astro](https://github.com/withastro/astro) | `1e4916333554cf3798d20cd33f4dbf91e2844891` | [`examples/basics/tsconfig.json`](https://github.com/withastro/astro/blob/1e4916333554cf3798d20cd33f4dbf91e2844891/examples/basics/tsconfig.json) | MIT | Byte-identical to the Astro starter tsconfig; attributed although it may be below the threshold of originality. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [decaporg/decap-cms](https://github.com/decaporg/decap-cms) | `1d5868347d3d8648d7d5f7f59fc1b46595881b29` | [`packages/decap-cms-widget-markdown/src/__tests__/renderer.spec.js`](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/packages/decap-cms-widget-markdown/src/__tests__/renderer.spec.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`packages/instantsearch-ui-components/src/components/chat/__tests__/ChatOverlayLayout.test.tsx`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/packages/instantsearch-ui-components/src/components/chat/__tests__/ChatOverlayLayout.test.tsx) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [chrisvel/tududi](https://github.com/chrisvel/tududi) | `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8` | [`frontend/components/Admin/__tests__/AdminUsersPage.test.tsx`](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/frontend/components/Admin/__tests__/AdminUsersPage.test.tsx) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`examples/CustomSingle.test.tsx`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/examples/CustomSingle.test.tsx) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [gpbl/react-day-picker](https://github.com/gpbl/react-day-picker) | `19033b454689bc761d9847301de429bbbb53eed4` | [`test/elements.ts`](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/test/elements.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/@expo/cli/e2e/playwright/prod/server-loader.test.ts`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/@expo/cli/e2e/playwright/prod/server-loader.test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/absence_anchor__check_json_absence_anchor.snap` | [rjsf-team/react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) | `ad0ea07428f8fde1ed1a958a2383861e242f14d7` | [`packages/core/test/LayoutGridField.test.tsx`](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/packages/core/test/LayoutGridField.test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/absence_anchor.rs. |
| `crates/spirolo/tests/snapshots/dead_skip__check_json_dead_skip.snap` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-ui-materialui/src/input/FileInput.spec.tsx`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-ui-materialui/src/input/FileInput.spec.tsx) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/dead_skip.rs. |
| `crates/spirolo/tests/snapshots/dead_skip__check_json_dead_skip.snap` | [marmelab/react-admin](https://github.com/marmelab/react-admin) | `99e8c52b7db1712c0afa4eeed533e4a713dfc1ec` | [`packages/ra-ui-materialui/src/input/ImageInput.spec.tsx`](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/packages/ra-ui-materialui/src/input/ImageInput.spec.tsx) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/dead_skip.rs. |
| `crates/spirolo/tests/snapshots/dead_skip__check_json_dead_skip.snap` | [gatsbyjs/gatsby](https://github.com/gatsbyjs/gatsby) | `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e` | [`integration-tests/structured-logging/__tests__/to-do.js`](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/integration-tests/structured-logging/__tests__/to-do.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/dead_skip.rs. |
| `crates/spirolo/tests/snapshots/dead_skip__check_json_dead_skip.snap` | [mobxjs/mobx](https://github.com/mobxjs/mobx) | `404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479` | [`packages/mobx/__tests__/base/typescript-tests.ts`](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/packages/mobx/__tests__/base/typescript-tests.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/dead_skip.rs. |
| `crates/spirolo/tests/snapshots/dead_skip__check_json_dead_skip.snap` | [radix-ui/primitives](https://github.com/radix-ui/primitives) | `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` | [`packages/react/slot/src/slot.test.tsx`](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/packages/react/slot/src/slot.test.tsx) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/dead_skip.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_agent_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_human_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_json_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test_propose.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__check_sarif_duplicate_test.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [Nozbe/WatermelonDB](https://github.com/Nozbe/WatermelonDB) | `5c463222ecd79928861385bac94b486fcc7623d4` | [`src/Database/test.js`](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/src/Database/test.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-nojs-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-nojs-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [remix-run/react-router](https://github.com/remix-run/react-router) | `54fd9629cc46fca255ba06fbf444bbc1126f2ac9` | [`integration/rsc/rsc-test.ts`](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/integration/rsc/rsc-test.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [mui/material-ui](https://github.com/mui/material-ui) | `6780195595c12252a130aa5ae5c5bd461229c6b7` | [`test/e2e-website/material-icons.spec.ts`](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/test/e2e-website/material-icons.spec.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`packages/web-components/src/components/accordion/__tests__/accordion-test.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/packages/web-components/src/components/accordion/__tests__/accordion-test.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. Copyright IBM Corp. 2025, 2026 |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-layout-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-layout-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/duplicate_test__explain_copy_paste.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/js/components/Box/__tests__/Box-test.tsx`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/js/components/Box/__tests__/Box-test.tsx) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/duplicate_test.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [grommet/grommet](https://github.com/grommet/grommet) | `fbb505b6b980b99415010318269833567ffc0d38` | [`src/setupTests.js`](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/src/setupTests.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [mantinedev/mantine](https://github.com/mantinedev/mantine) | `f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4` | [`jsdom.mocks.ts`](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/jsdom.mocks.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [algolia/instantsearch](https://github.com/algolia/instantsearch) | `3892320b3aa642121c1b610441017f41ef947b2f` | [`tests/utils/setupTests.ts`](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/tests/utils/setupTests.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [akveo/react-native-ui-kitten](https://github.com/akveo/react-native-ui-kitten) | `3f4e3d0467c418f0f90defb873e736334000cc28` | [`jest.setup.js`](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/jest.setup.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [react/react](https://github.com/react/react) | `7c6ac13e19fef500b7f669a16bbd01ecc95965ca` | [`packages/react-devtools-shared/src/__tests__/setupTests.js`](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/packages/react-devtools-shared/src/__tests__/setupTests.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. Copyright (c) Meta Platforms, Inc. and affiliates. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`config/jest-config-carbon/setup/setupAfterEnv.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/config/jest-config-carbon/setup/setupAfterEnv.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. Copyright IBM Corp. 2016, 2023 |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) | `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d` | [`config/jest-config-carbon/index.js`](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/config/jest-config-carbon/index.js) | Apache-2.0 | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. Copyright IBM Corp. 2018, 2023 |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/expo-module-scripts/jest-preset.cjs`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/expo-module-scripts/jest-preset.cjs) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [expo/expo](https://github.com/expo/expo) | `500d25dea3746c8ceeb751b3c55f432b269be410` | [`packages/jest-expo/universal/jest-preset.js`](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/packages/jest-expo/universal/jest-preset.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [fuma-nama/fumadocs](https://github.com/fuma-nama/fumadocs) | `bb43b610100da6324f561ec3b4049e59f1a40663` | [`packages/create-app/vitest.config.ts`](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/packages/create-app/vitest.config.ts) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/plugin-oxc/jest.config.js`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/plugin-oxc/jest.config.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [software-mansion/react-native-reanimated](https://github.com/software-mansion/react-native-reanimated) | `b603aacf3ebd0fe368125faaab5c467334687ddb` | [`packages/react-native-worklets/plugin/jest.config.js`](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/plugin/jest.config.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |
| `crates/spirolo/tests/snapshots/fail_closed_setup__check_json_fail_closed_setup.snap` | [react-hook-form/react-hook-form](https://github.com/react-hook-form/react-hook-form) | `c4dc4456e60c20833a2c70931128224e6cb13176` | [`scripts/jest/jest.config.js`](https://github.com/react-hook-form/react-hook-form/blob/c4dc4456e60c20833a2c70931128224e6cb13176/scripts/jest/jest.config.js) | MIT | Expected analyzer output from the adapted inline fixture excerpts in crates/spirolo/tests/fail_closed_setup.rs. |

### Withheld pending provenance

- `crates/spirolo-frameworks/tests/snapshot.rs`: Contains a Vue snapshot excerpt with no recorded upstream commit. Withheld until the pin and licence are verified.

Corpus recordings (`corpus/**/recorded/**`) are excluded from the public export.

### akveo__react-native-ui-kitten/LICENSE.txt

[MIT: LICENSE.txt](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/LICENSE.txt)

````text
The MIT License (MIT)

Copyright (c) 2016 akveo.com
Copyright (c) 2024-2026 Vlad Bataev and UI Kitten Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### akveo__react-native-ui-kitten/NOTICE

[NOTICE: NOTICE](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/NOTICE)

````text
UI Kitten
Copyright (c) 2016 akveo.com
Copyright (c) 2024-2026 Vlad Bataev and UI Kitten Contributors

The Eva Design System processor (src/processor/) is derived from
@eva-design/processor, Copyright (c) akveo.com, licensed under the MIT License.

The Eva mapping packages (src/eva/, src/material/) are derived from
@eva-design/eva and @eva-design/material, Copyright (c) akveo.com,
licensed under the MIT License.

The icon artwork in src/eva-icons/icons/ is generated from Eva Icons 1.1.3
(https://github.com/akveo/eva-icons), Copyright (c) 2018 Akveo, licensed under
the MIT License.

````

### akveo__react-native-ui-kitten/src/eva-icons/LICENSE

[MIT: src/eva-icons/LICENSE](https://github.com/akveo/react-native-ui-kitten/blob/3f4e3d0467c418f0f90defb873e736334000cc28/src/eva-icons/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2016 akveo.com
Copyright (c) 2024-2026 Vlad Bataev and UI Kitten Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

The icon artwork in this package (the modules under `icons/`) is derived from
Eva Icons 1.1.3, https://github.com/akveo/eva-icons, distributed under the
following license:

The MIT License (MIT)

Copyright (c) 2018 Akveo.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


````

### algolia__instantsearch/LICENSE

[MIT: LICENSE](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2015-present Algolia, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### algolia__instantsearch/packages/create-instantsearch-app/LICENSE

[MIT: packages/create-instantsearch-app/LICENSE](https://github.com/algolia/instantsearch/blob/3892320b3aa642121c1b610441017f41ef947b2f/packages/create-instantsearch-app/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2015-present Algolia, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### boardgameio__boardgame.io/LICENSE

[MIT: LICENSE](https://github.com/boardgameio/boardgame.io/blob/5e9a2c94bde803fae8b081958c406c4d0a7be8ae/LICENSE)

````text
MIT License

Copyright (c) 2017 The boardgame.io Authors.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### callstack__react-native-paper/LICENSE.md

[MIT: LICENSE.md](https://github.com/callstack/react-native-paper/blob/4ffc20ae4ac463cff70f50e22bc627531ec59467/LICENSE.md)

````text
MIT License

Copyright (c) 2017 Callstack

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### carbon-design-system__carbon/LICENSE

[Apache-2.0: LICENSE](https://github.com/carbon-design-system/carbon/blob/40f8d5cfaed733feefa2e7c39b8e9eea1795d97d/LICENSE)

````text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS

   APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "{}"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

   Copyright 2015 IBM Corp.

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.

````

### chakra-ui__chakra-ui/LICENSE

[MIT: LICENSE](https://github.com/chakra-ui/chakra-ui/blob/961161428b8c59157ad921dd23303b73c294d73f/LICENSE)

````text
MIT License

Copyright (c) 2019 Chakra Systems Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

````

### ChatGPTNextWeb__NextChat/LICENSE

[MIT: LICENSE](https://github.com/ChatGPTNextWeb/NextChat/blob/defdcdb55d850cd12c4c657eb83729fd66e215c0/LICENSE)

````text
MIT License

Copyright (c) 2023-2025 NextChat

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### chrisvel__tududi/LICENSE

[MIT: LICENSE](https://github.com/chrisvel/tududi/blob/1781c4c170b8e06f0d9d78e774bde9c95b3be1a8/LICENSE)

````text
The MIT License (MIT)

Copyright 2024, 2025 Tududi Developers

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

````

### decaporg__decap-cms/LICENSE

[MIT: LICENSE](https://github.com/decaporg/decap-cms/blob/1d5868347d3d8648d7d5f7f59fc1b46595881b29/LICENSE)

````text
Copyright (c) 2016 Netlify <decap@p-m.si>

MIT License

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

````

### expo__expo/docs/LICENSE

[MIT: docs/LICENSE](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/docs/LICENSE)

````text
MIT License

Copyright (c) 2017 650 Industries, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### expo__expo/LICENSE

[MIT: LICENSE](https://github.com/expo/expo/blob/500d25dea3746c8ceeb751b3c55f432b269be410/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2015-present 650 Industries, Inc. (aka Expo)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### extension-js__extension.js/LICENSE

[MIT: LICENSE](https://github.com/extension-js/extension.js/blob/8143b84d990a50225a94ea6532acfa1e32c423e1/LICENSE)

````text
MIT License

Copyright (c) 2020 Cezar Augusto <boss@cezaraugusto.net> (https://cezaraugusto.com)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### fastrepl__anarlog/LICENSE

[MIT: LICENSE](https://github.com/fastrepl/anarlog/blob/3de75bfb22d876be27e15a038e3e319091b6d016/LICENSE)

````text
MIT License

Copyright (c) 2023-present Fastrepl, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### fuma-nama__fumadocs/LICENSE

[MIT: LICENSE](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/LICENSE)

````text
MIT License

Copyright (c) 2023 Fuma

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### fuma-nama__fumadocs/packages/image-size/LICENSE

[MIT: packages/image-size/LICENSE](https://github.com/fuma-nama/fumadocs/blob/bb43b610100da6324f561ec3b4049e59f1a40663/packages/image-size/LICENSE)

````text
MIT License

Copyright (c) 2026 Fuma Nama

This package is a fork of probe-image-size (https://github.com/nodeca/probe-image-size).
The image format parsers are derived from that project:

Copyright (c) 2017 Nodeca

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### gatsbyjs__gatsby/e2e-tests/development-runtime/LICENSE

[MIT: e2e-tests/development-runtime/LICENSE](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/e2e-tests/development-runtime/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2015 gatsbyjs

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


````

### gatsbyjs__gatsby/integration-tests/artifacts/LICENSE

[MIT: integration-tests/artifacts/LICENSE](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/integration-tests/artifacts/LICENSE)

````text
MIT License

Copyright (c) 2018 gatsbyjs

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### gatsbyjs__gatsby/integration-tests/structured-logging/LICENSE

[MIT: integration-tests/structured-logging/LICENSE](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/integration-tests/structured-logging/LICENSE)

````text
MIT License

Copyright (c) 2019 gatsbyjs

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### gatsbyjs__gatsby/LICENSE

[MIT: LICENSE](https://github.com/gatsbyjs/gatsby/blob/8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2015 Gatsbyjs

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


````

### gpbl__react-day-picker/LICENSE

[MIT: LICENSE](https://github.com/gpbl/react-day-picker/blob/19033b454689bc761d9847301de429bbbb53eed4/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2014-2026 Giampaolo Bellavite <io@gpbl.dev> and contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### grommet__grommet/LICENSE

[Apache-2.0: LICENSE](https://github.com/grommet/grommet/blob/fbb505b6b980b99415010318269833567ffc0d38/LICENSE)

````text
                              Apache License
                         Version 2.0, January 2004
                      http://www.apache.org/licenses/

 TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

 1. Definitions.

    "License" shall mean the terms and conditions for use, reproduction,
    and distribution as defined by Sections 1 through 9 of this document.

    "Licensor" shall mean the copyright owner or entity authorized by
    the copyright owner that is granting the License.

    "Legal Entity" shall mean the union of the acting entity and all
    other entities that control, are controlled by, or are under common
    control with that entity. For the purposes of this definition,
    "control" means (i) the power, direct or indirect, to cause the
    direction or management of such entity, whether by contract or
    otherwise, or (ii) ownership of fifty percent (50%) or more of the
    outstanding shares, or (iii) beneficial ownership of such entity.

    "You" (or "Your") shall mean an individual or Legal Entity
    exercising permissions granted by this License.

    "Source" form shall mean the preferred form for making modifications,
    including but not limited to software source code, documentation
    source, and configuration files.

    "Object" form shall mean any form resulting from mechanical
    transformation or translation of a Source form, including but
    not limited to compiled object code, generated documentation,
    and conversions to other media types.

    "Work" shall mean the work of authorship, whether in Source or
    Object form, made available under the License, as indicated by a
    copyright notice that is included in or attached to the work
    (an example is provided in the Appendix below).

    "Derivative Works" shall mean any work, whether in Source or Object
    form, that is based on (or derived from) the Work and for which the
    editorial revisions, annotations, elaborations, or other modifications
    represent, as a whole, an original work of authorship. For the purposes
    of this License, Derivative Works shall not include works that remain
    separable from, or merely link (or bind by name) to the interfaces of,
    the Work and Derivative Works thereof.

    "Contribution" shall mean any work of authorship, including
    the original version of the Work and any modifications or additions
    to that Work or Derivative Works thereof, that is intentionally
    submitted to Licensor for inclusion in the Work by the copyright owner
    or by an individual or Legal Entity authorized to submit on behalf of
    the copyright owner. For the purposes of this definition, "submitted"
    means any form of electronic, verbal, or written communication sent
    to the Licensor or its representatives, including but not limited to
    communication on electronic mailing lists, source code control systems,
    and issue tracking systems that are managed by, or on behalf of, the
    Licensor for the purpose of discussing and improving the Work, but
    excluding communication that is conspicuously marked or otherwise
    designated in writing by the copyright owner as "Not a Contribution."

    "Contributor" shall mean Licensor and any individual or Legal Entity
    on behalf of whom a Contribution has been received by Licensor and
    subsequently incorporated within the Work.

 2. Grant of Copyright License. Subject to the terms and conditions of
    this License, each Contributor hereby grants to You a perpetual,
    worldwide, non-exclusive, no-charge, royalty-free, irrevocable
    copyright license to reproduce, prepare Derivative Works of,
    publicly display, publicly perform, sublicense, and distribute the
    Work and such Derivative Works in Source or Object form.

 3. Grant of Patent License. Subject to the terms and conditions of
    this License, each Contributor hereby grants to You a perpetual,
    worldwide, non-exclusive, no-charge, royalty-free, irrevocable
    (except as stated in this section) patent license to make, have made,
    use, offer to sell, sell, import, and otherwise transfer the Work,
    where such license applies only to those patent claims licensable
    by such Contributor that are necessarily infringed by their
    Contribution(s) alone or by combination of their Contribution(s)
    with the Work to which such Contribution(s) was submitted. If You
    institute patent litigation against any entity (including a
    cross-claim or counterclaim in a lawsuit) alleging that the Work
    or a Contribution incorporated within the Work constitutes direct
    or contributory patent infringement, then any patent licenses
    granted to You under this License for that Work shall terminate
    as of the date such litigation is filed.

 4. Redistribution. You may reproduce and distribute copies of the
    Work or Derivative Works thereof in any medium, with or without
    modifications, and in Source or Object form, provided that You
    meet the following conditions:

    (a) You must give any other recipients of the Work or
        Derivative Works a copy of this License; and

    (b) You must cause any modified files to carry prominent notices
        stating that You changed the files; and

    (c) You must retain, in the Source form of any Derivative Works
        that You distribute, all copyright, patent, trademark, and
        attribution notices from the Source form of the Work,
        excluding those notices that do not pertain to any part of
        the Derivative Works; and

    (d) If the Work includes a "NOTICE" text file as part of its
        distribution, then any Derivative Works that You distribute must
        include a readable copy of the attribution notices contained
        within such NOTICE file, excluding those notices that do not
        pertain to any part of the Derivative Works, in at least one
        of the following places: within a NOTICE text file distributed
        as part of the Derivative Works; within the Source form or
        documentation, if provided along with the Derivative Works; or,
        within a display generated by the Derivative Works, if and
        wherever such third-party notices normally appear. The contents
        of the NOTICE file are for informational purposes only and
        do not modify the License. You may add Your own attribution
        notices within Derivative Works that You distribute, alongside
        or as an addendum to the NOTICE text from the Work, provided
        that such additional attribution notices cannot be construed
        as modifying the License.

    You may add Your own copyright statement to Your modifications and
    may provide additional or different license terms and conditions
    for use, reproduction, or distribution of Your modifications, or
    for any such Derivative Works as a whole, provided Your use,
    reproduction, and distribution of the Work otherwise complies with
    the conditions stated in this License.

 5. Submission of Contributions. Unless You explicitly state otherwise,
    any Contribution intentionally submitted for inclusion in the Work
    by You to the Licensor shall be under the terms and conditions of
    this License, without any additional terms or conditions.
    Notwithstanding the above, nothing herein shall supersede or modify
    the terms of any separate license agreement you may have executed
    with Licensor regarding such Contributions.

 6. Trademarks. This License does not grant permission to use the trade
    names, trademarks, service marks, or product names of the Licensor,
    except as required for reasonable and customary use in describing the
    origin of the Work and reproducing the content of the NOTICE file.

 7. Disclaimer of Warranty. Unless required by applicable law or
    agreed to in writing, Licensor provides the Work (and each
    Contributor provides its Contributions) on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
    implied, including, without limitation, any warranties or conditions
    of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
    PARTICULAR PURPOSE. You are solely responsible for determining the
    appropriateness of using or redistributing the Work and assume any
    risks associated with Your exercise of permissions under this License.

 8. Limitation of Liability. In no event and under no legal theory,
    whether in tort (including negligence), contract, or otherwise,
    unless required by applicable law (such as deliberate and grossly
    negligent acts) or agreed to in writing, shall any Contributor be
    liable to You for damages, including any direct, indirect, special,
    incidental, or consequential damages of any character arising as a
    result of this License or out of the use or inability to use the
    Work (including but not limited to damages for loss of goodwill,
    work stoppage, computer failure or malfunction, or any and all
    other commercial damages or losses), even if such Contributor
    has been advised of the possibility of such damages.

 9. Accepting Warranty or Additional Liability. While redistributing
    the Work or Derivative Works thereof, You may choose to offer,
    and charge a fee for, acceptance of support, warranty, indemnity,
    or other liability obligations and/or rights consistent with this
    License. However, in accepting such obligations, You may act only
    on Your own behalf and on Your sole responsibility, not on behalf
    of any other Contributor, and only if You agree to indemnify,
    defend, and hold each Contributor harmless for any liability
    incurred by, or claims asserted against, such Contributor by reason
    of your accepting any such warranty or additional liability.

 END OF TERMS AND CONDITIONS
````

### hyperdxio__hyperdx/LICENSE

[MIT: LICENSE](https://github.com/hyperdxio/hyperdx/blob/19182e7e21d0dd987bee43f5a623acf4a7785e30/LICENSE)

````text
MIT License

Copyright (c) 2023 DeploySentinel, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### mantinedev__mantine/LICENSE

[MIT: LICENSE](https://github.com/mantinedev/mantine/blob/f38933cb4f1c534600f4ff59ee3ddbb4685a4bc4/LICENSE)

````text
MIT License

Copyright (c) 2021 Vitaly Rtishchev

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### marmelab__react-admin/LICENSE.md

[MIT: LICENSE.md](https://github.com/marmelab/react-admin/blob/99e8c52b7db1712c0afa4eeed533e4a713dfc1ec/LICENSE.md)

````text
The MIT License (MIT)

Copyright (c) 2016-present, Francois Zaninotto, Marmelab

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NON INFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### middleapi__orpc/LICENSE

[MIT: LICENSE](https://github.com/middleapi/orpc/blob/63b20537dbb6ee91a4d68c6d81fa1f53ab114964/LICENSE)

````text
MIT License

Copyright (c) 2023 oRPC

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
````

### midwayjs__midway/LICENSE

[MIT: LICENSE](https://github.com/midwayjs/midway/blob/bd306bb74b025168b4a7386cff78aacf7862a81e/LICENSE)

````text
MIT License

Copyright (c) 2013 - Now midwayjs

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### mobxjs__mobx/LICENSE

[MIT: LICENSE](https://github.com/mobxjs/mobx/blob/404c6e08f3ed38fefdd5b37a0ba21d31cb2bf479/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2015 Michel Weststrate

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### mui__material-ui/LICENSE

[MIT: LICENSE](https://github.com/mui/material-ui/blob/6780195595c12252a130aa5ae5c5bd461229c6b7/LICENSE)

````text
MIT License

Copyright (c) 2014 Call-Em-All

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### necolas__react-native-web/LICENSE

[MIT: LICENSE](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/LICENSE)

````text
MIT License

Copyright (c) Nicolas Gallagher.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### necolas__react-native-web/packages/react-native-web/LICENSE

[MIT: packages/react-native-web/LICENSE](https://github.com/necolas/react-native-web/blob/f1b53224b242d9d8dad4b647e0b6f0395a0533ae/packages/react-native-web/LICENSE)

````text
MIT License

Copyright (c) Nicolas Gallagher.
Copyright (c) Meta Platforms, Inc. and affiliates.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### notionnext-org__NotionNext/LICENSE

[MIT: LICENSE](https://github.com/notionnext-org/NotionNext/blob/50e77e069ec48ebd3d641888c13cd5ec6842c3d2/LICENSE)

````text
MIT License

Copyright (c) 2021-present, tangly1024

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### Nozbe__WatermelonDB/LICENSE

[MIT: LICENSE](https://github.com/Nozbe/WatermelonDB/blob/5c463222ecd79928861385bac94b486fcc7623d4/LICENSE)

````text
MIT License

Copyright (c) Nozbe

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### plouc__nivo/LICENSE.md

[MIT: LICENSE.md](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/LICENSE.md)

````text
Copyright (c) Raphaël Benitte

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
````

### plouc__nivo/packages/arcs/LICENSE.md

[MIT: packages/arcs/LICENSE.md](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/arcs/LICENSE.md)

````text
Copyright (c) Raphaël Benitte

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
````

### pmndrs__react-spring/LICENSE

[MIT: LICENSE](https://github.com/pmndrs/react-spring/blob/d9cd058a07c7ac5b576f40a5f4312f4471897661/LICENSE)

````text
MIT License

Copyright (c) 2018-present Paul Henschel, react-spring, all contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### pmndrs__react-spring/packages/core/LICENSE

[MIT: packages/core/LICENSE](https://github.com/pmndrs/react-spring/blob/d9cd058a07c7ac5b576f40a5f4312f4471897661/packages/core/LICENSE)

````text
MIT License

Copyright (c) 2018-present Paul Henschel, react-spring, all contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### pmndrs__valtio/LICENSE

[MIT: LICENSE](https://github.com/pmndrs/valtio/blob/6c4af4ba642b959e86e698c495176fdce8646c05/LICENSE)

````text
MIT License

Copyright (c) 2020 Poimandres

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### radix-ui__primitives/LICENSE

[MIT: LICENSE](https://github.com/radix-ui/primitives/blob/f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae/LICENSE)

````text
MIT License

Copyright (c) 2022 WorkOS

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### react__metro/LICENSE

[MIT: LICENSE](https://github.com/react/metro/blob/5793c17017825d1d59191d080114365e1da569ae/LICENSE)

````text
MIT License

Copyright (c) Meta Platforms, Inc. and affiliates.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### react__react/LICENSE

[MIT: LICENSE](https://github.com/react/react/blob/7c6ac13e19fef500b7f669a16bbd01ecc95965ca/LICENSE)

````text
MIT License

Copyright (c) Meta Platforms, Inc. and affiliates.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### react-hook-form__react-hook-form/LICENSE

[MIT: LICENSE](https://github.com/react-hook-form/react-hook-form/blob/c4dc4456e60c20833a2c70931128224e6cb13176/LICENSE)

````text
MIT License

Copyright (c) 2019-present Beier(Bill) Luo

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### remix-run__react-router/LICENSE.md

[MIT: LICENSE.md](https://github.com/remix-run/react-router/blob/54fd9629cc46fca255ba06fbf444bbc1126f2ac9/LICENSE.md)

````text
MIT License

Copyright (c) React Training LLC 2015-2019
Copyright (c) Remix Software Inc. 2020-2021
Copyright (c) Shopify Inc. 2022-2023

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### rjsf-team__react-jsonschema-form/LICENSE.md

[Apache-2.0: LICENSE.md](https://github.com/rjsf-team/react-jsonschema-form/blob/ad0ea07428f8fde1ed1a958a2383861e242f14d7/LICENSE.md)

````text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1.  Definitions.

    "License" shall mean the terms and conditions for use, reproduction,
    and distribution as defined by Sections 1 through 9 of this document.

    "Licensor" shall mean the copyright owner or entity authorized by
    the copyright owner that is granting the License.

    "Legal Entity" shall mean the union of the acting entity and all
    other entities that control, are controlled by, or are under common
    control with that entity. For the purposes of this definition,
    "control" means (i) the power, direct or indirect, to cause the
    direction or management of such entity, whether by contract or
    otherwise, or (ii) ownership of fifty percent (50%) or more of the
    outstanding shares, or (iii) beneficial ownership of such entity.

    "You" (or "Your") shall mean an individual or Legal Entity
    exercising permissions granted by this License.

    "Source" form shall mean the preferred form for making modifications,
    including but not limited to software source code, documentation
    source, and configuration files.

    "Object" form shall mean any form resulting from mechanical
    transformation or translation of a Source form, including but
    not limited to compiled object code, generated documentation,
    and conversions to other media types.

    "Work" shall mean the work of authorship, whether in Source or
    Object form, made available under the License, as indicated by a
    copyright notice that is included in or attached to the work
    (an example is provided in the Appendix below).

    "Derivative Works" shall mean any work, whether in Source or Object
    form, that is based on (or derived from) the Work and for which the
    editorial revisions, annotations, elaborations, or other modifications
    represent, as a whole, an original work of authorship. For the purposes
    of this License, Derivative Works shall not include works that remain
    separable from, or merely link (or bind by name) to the interfaces of,
    the Work and Derivative Works thereof.

    "Contribution" shall mean any work of authorship, including
    the original version of the Work and any modifications or additions
    to that Work or Derivative Works thereof, that is intentionally
    submitted to Licensor for inclusion in the Work by the copyright owner
    or by an individual or Legal Entity authorized to submit on behalf of
    the copyright owner. For the purposes of this definition, "submitted"
    means any form of electronic, verbal, or written communication sent
    to the Licensor or its representatives, including but not limited to
    communication on electronic mailing lists, source code control systems,
    and issue tracking systems that are managed by, or on behalf of, the
    Licensor for the purpose of discussing and improving the Work, but
    excluding communication that is conspicuously marked or otherwise
    designated in writing by the copyright owner as "Not a Contribution."

    "Contributor" shall mean Licensor and any individual or Legal Entity
    on behalf of whom a Contribution has been received by Licensor and
    subsequently incorporated within the Work.

2.  Grant of Copyright License. Subject to the terms and conditions of
    this License, each Contributor hereby grants to You a perpetual,
    worldwide, non-exclusive, no-charge, royalty-free, irrevocable
    copyright license to reproduce, prepare Derivative Works of,
    publicly display, publicly perform, sublicense, and distribute the
    Work and such Derivative Works in Source or Object form.

3.  Grant of Patent License. Subject to the terms and conditions of
    this License, each Contributor hereby grants to You a perpetual,
    worldwide, non-exclusive, no-charge, royalty-free, irrevocable
    (except as stated in this section) patent license to make, have made,
    use, offer to sell, sell, import, and otherwise transfer the Work,
    where such license applies only to those patent claims licensable
    by such Contributor that are necessarily infringed by their
    Contribution(s) alone or by combination of their Contribution(s)
    with the Work to which such Contribution(s) was submitted. If You
    institute patent litigation against any entity (including a
    cross-claim or counterclaim in a lawsuit) alleging that the Work
    or a Contribution incorporated within the Work constitutes direct
    or contributory patent infringement, then any patent licenses
    granted to You under this License for that Work shall terminate
    as of the date such litigation is filed.

4.  Redistribution. You may reproduce and distribute copies of the
    Work or Derivative Works thereof in any medium, with or without
    modifications, and in Source or Object form, provided that You
    meet the following conditions:

    (a) You must give any other recipients of the Work or
    Derivative Works a copy of this License; and

    (b) You must cause any modified files to carry prominent notices
    stating that You changed the files; and

    (c) You must retain, in the Source form of any Derivative Works
    that You distribute, all copyright, patent, trademark, and
    attribution notices from the Source form of the Work,
    excluding those notices that do not pertain to any part of
    the Derivative Works; and

    (d) If the Work includes a "NOTICE" text file as part of its
    distribution, then any Derivative Works that You distribute must
    include a readable copy of the attribution notices contained
    within such NOTICE file, excluding those notices that do not
    pertain to any part of the Derivative Works, in at least one
    of the following places: within a NOTICE text file distributed
    as part of the Derivative Works; within the Source form or
    documentation, if provided along with the Derivative Works; or,
    within a display generated by the Derivative Works, if and
    wherever such third-party notices normally appear. The contents
    of the NOTICE file are for informational purposes only and
    do not modify the License. You may add Your own attribution
    notices within Derivative Works that You distribute, alongside
    or as an addendum to the NOTICE text from the Work, provided
    that such additional attribution notices cannot be construed
    as modifying the License.

    You may add Your own copyright statement to Your modifications and
    may provide additional or different license terms and conditions
    for use, reproduction, or distribution of Your modifications, or
    for any such Derivative Works as a whole, provided Your use,
    reproduction, and distribution of the Work otherwise complies with
    the conditions stated in this License.

5.  Submission of Contributions. Unless You explicitly state otherwise,
    any Contribution intentionally submitted for inclusion in the Work
    by You to the Licensor shall be under the terms and conditions of
    this License, without any additional terms or conditions.
    Notwithstanding the above, nothing herein shall supersede or modify
    the terms of any separate license agreement you may have executed
    with Licensor regarding such Contributions.

6.  Trademarks. This License does not grant permission to use the trade
    names, trademarks, service marks, or product names of the Licensor,
    except as required for reasonable and customary use in describing the
    origin of the Work and reproducing the content of the NOTICE file.

7.  Disclaimer of Warranty. Unless required by applicable law or
    agreed to in writing, Licensor provides the Work (and each
    Contributor provides its Contributions) on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
    implied, including, without limitation, any warranties or conditions
    of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
    PARTICULAR PURPOSE. You are solely responsible for determining the
    appropriateness of using or redistributing the Work and assume any
    risks associated with Your exercise of permissions under this License.

8.  Limitation of Liability. In no event and under no legal theory,
    whether in tort (including negligence), contract, or otherwise,
    unless required by applicable law (such as deliberate and grossly
    negligent acts) or agreed to in writing, shall any Contributor be
    liable to You for damages, including any direct, indirect, special,
    incidental, or consequential damages of any character arising as a
    result of this License or out of the use or inability to use the
    Work (including but not limited to damages for loss of goodwill,
    work stoppage, computer failure or malfunction, or any and all
    other commercial damages or losses), even if such Contributor
    has been advised of the possibility of such damages.

9.  Accepting Warranty or Additional Liability. While redistributing
    the Work or Derivative Works thereof, You may choose to offer,
    and charge a fee for, acceptance of support, warranty, indemnity,
    or other liability obligations and/or rights consistent with this
    License. However, in accepting such obligations, You may act only
    on Your own behalf and on Your sole responsibility, not on behalf
    of any other Contributor, and only if You agree to indemnify,
    defend, and hold each Contributor harmless for any liability
    incurred by, or claims asserted against, such Contributor by reason
    of your accepting any such warranty or additional liability.

END OF TERMS AND CONDITIONS

APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

Copyright 2015-2026 rjsf-team

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

````

### shuding__nextra/LICENSE

[MIT: LICENSE](https://github.com/shuding/nextra/blob/d6e80e1dd627b781429a6ee989b15ebba688c8ea/LICENSE)

````text
MIT License

Copyright (c) 2020 Shu Ding

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### software-mansion__react-native-reanimated/LICENSE

[MIT: LICENSE](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/LICENSE)

````text
The MIT License (MIT)

Copyright (c) 2016 Software Mansion <swmansion.com>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### software-mansion__react-native-reanimated/packages/react-native-worklets/LICENSE

[MIT: packages/react-native-worklets/LICENSE](https://github.com/software-mansion/react-native-reanimated/blob/b603aacf3ebd0fe368125faaab5c467334687ddb/packages/react-native-worklets/LICENSE)

````text
MIT License

Copyright (c) 2024 nobody
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### usekaneo__kaneo/LICENSE

[MIT: LICENSE](https://github.com/usekaneo/kaneo/blob/afe97be2473e0cd97fe4a63e526fef9ac4df85b0/LICENSE)

````text
MIT License

Copyright (c) 2024 Andrej Acevski

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

````

### withastro__astro/LICENSE

[MIT: LICENSE](https://github.com/withastro/astro/blob/1e4916333554cf3798d20cd33f4dbf91e2844891/LICENSE)

````text
MIT License

Copyright (c) 2021 Fred K. Schott

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

"""
This license applies to parts of the `packages/create-astro` and `packages/astro` subdirectories originating from the https://github.com/sveltejs/kit repository:

Copyright (c) 2020 [these people](https://github.com/sveltejs/kit/graphs/contributors)

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
"""

"""
This license applies to parts of the `packages/create-astro` and `packages/astro` subdirectories originating from the https://github.com/vitejs/vite repository:

MIT License

Copyright (c) 2019-present, Yuxi (Evan) You and Vite contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

````

<!-- END fixture notices -->
