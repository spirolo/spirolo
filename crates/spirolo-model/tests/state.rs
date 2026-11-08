//! State resets in real dev-corpus source, through the public model builder.

use std::path::Path;

use spirolo_model::{BuildOptions, Hook, Model};

#[test]
fn a_named_arrow_reset_is_registered_in_the_hooks_scope() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/named-reset");
    let model = Model::build(&root, &BuildOptions::default());
    let resets: Vec<_> = model
        .state_accesses
        .iter()
        .filter(|access| {
            access.file == "agentService.test.ts"
                && access.write
                && access.hook == Some(Hook::BeforeEach)
        })
        .map(|access| (access.binding.as_str(), access.span.line))
        .collect();
    assert_eq!(resets, [("mockCounterAdds", 40), ("mockSpanAttrs", 40)]);
}

#[test]
fn a_named_function_reset_is_registered_in_the_hooks_scope() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/named-reset");
    let model = Model::build(&root, &BuildOptions::default());
    let resets: Vec<_> = model
        .state_accesses
        .iter()
        .filter(|access| {
            access.file == "useSidePanelStack.test.tsx"
                && access.write
                && access.hook == Some(Hook::BeforeEach)
        })
        .map(|access| (access.binding.as_str(), access.span.line))
        .collect();
    assert_eq!(resets, [("mockQueryStore", 121), ("mockSetters", 121)]);
}
