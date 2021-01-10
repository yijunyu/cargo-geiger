use super::{
    file_forbids_unsafe, is_test_fn, is_test_mod, IncludeTests, RsFileMetrics,
};
use syn::{visit, Expr, ImplItemMethod, ItemFn, ItemImpl, ItemMod, ItemTrait};
use quote::ToTokens;
use std::path::{Path, PathBuf};

pub struct GeigerSynVisitor {
    /// Count unsafe usage inside tests
    include_tests: IncludeTests,

    /// The resulting data from a single file scan.
    pub metrics: RsFileMetrics,

    /// The number of nested unsafe scopes that the GeigerSynVisitor are
    /// currently in. For example, if the visitor is inside an unsafe function
    /// and inside an unnecessary unsafe block inside that function, then this
    /// number should be 2. If the visitor is outside unsafe scopes, in a safe
    /// scope, this number should be 0.
    /// This is needed since unsafe scopes can be nested and we need to know
    /// when we leave the outmost unsafe scope and get back into a safe scope.
    unsafe_scopes: u32,

    pub path: String,
}

fn get_filename(path: &Path) -> String {
    let mut p2 = PathBuf::new();
    for ancestor in path.ancestors() {
        match ancestor.file_stem() {
            Some(a) => {
                if a.to_os_string() == "src" {
                    let p1 = PathBuf::new().join(ancestor.parent().expect("not empty").file_stem().expect("not empty").to_os_string());
                    let pp = p1.join(p2.parent().expect("not empty"));
                    std::fs::create_dir_all(PathBuf::new().join("../safe").join(&pp)).unwrap();
                    std::fs::create_dir_all(PathBuf::new().join("../unsafe").join(&pp)).unwrap();
                    return pp.join(path.file_stem().expect("not empty").to_os_string()).to_string_lossy().to_string();
                }
                p2 = Path::new(a).to_path_buf().join(p2);
            },
            None => {
                break
            }
        }
    }
    String::from("")
}

impl GeigerSynVisitor {

    pub fn new(include_tests: IncludeTests, path: &Path) -> Self {
        GeigerSynVisitor {
            include_tests,
            metrics: Default::default(),
            unsafe_scopes: 0,
            path: get_filename(path),
        }
    }

    pub fn enter_unsafe_scope(&mut self) {
        self.unsafe_scopes += 1;
    }

    pub fn exit_unsafe_scope(&mut self) {
        self.unsafe_scopes -= 1;
    }

}

impl<'ast> visit::Visit<'ast> for GeigerSynVisitor {
    fn visit_file(&mut self, i: &'ast syn::File) {
        self.metrics.forbids_unsafe = file_forbids_unsafe(i);
        syn::visit::visit_file(self, i);
    }

    /// Free-standing functions
    fn visit_item_fn(&mut self, item_fn: &ItemFn) {
        if IncludeTests::No == self.include_tests && is_test_fn(item_fn) {
            return;
        }
        let mut filename = format!("../safe/{}-{}.rs", self.path, item_fn.sig.ident.to_string());
        if item_fn.sig.unsafety.is_some() {
            filename = format!("../unsafe/{}-{}.rs", self.path, item_fn.sig.ident.to_string());
            self.enter_unsafe_scope();
        }
        if self.path != "" {
            std::fs::write(filename, item_fn.into_token_stream().to_string().as_bytes()).unwrap();
        }
        self.metrics
            .counters
            .functions
            .count(item_fn.sig.unsafety.is_some());
        visit::visit_item_fn(self, item_fn);
        if item_fn.sig.unsafety.is_some() {
            self.exit_unsafe_scope()
        }
    }

    fn visit_expr(&mut self, i: &Expr) {
        // Total number of expressions of any type
        match i {
            Expr::Unsafe(i) => {
                self.enter_unsafe_scope();
                visit::visit_expr_unsafe(self, i);
                self.exit_unsafe_scope();
            }
            Expr::Path(_) | Expr::Lit(_) => {
                // Do not count. The expression `f(x)` should count as one
                // expression, not three.
            }
            other => {
                // TODO: Print something pretty here or gather the data for later
                // printing.
                // if self.verbosity == Verbosity::Verbose && self.unsafe_scopes > 0 {
                //     println!("{:#?}", other);
                // }
                self.metrics.counters.exprs.count(self.unsafe_scopes > 0);
                visit::visit_expr(self, other);
            }
        }
    }

    fn visit_item_mod(&mut self, i: &ItemMod) {
        if IncludeTests::No == self.include_tests && is_test_mod(i) {
            return;
        }
        visit::visit_item_mod(self, i);
    }

    fn visit_item_impl(&mut self, i: &ItemImpl) {
        // unsafe trait impl's
        self.metrics.counters.item_impls.count(i.unsafety.is_some());
        visit::visit_item_impl(self, i);
    }

    fn visit_item_trait(&mut self, i: &ItemTrait) {
        // Unsafe traits
        self.metrics
            .counters
            .item_traits
            .count(i.unsafety.is_some());
        visit::visit_item_trait(self, i);
    }

    fn visit_impl_item_method(&mut self, i: &ImplItemMethod) {
        if i.sig.unsafety.is_some() {
            self.enter_unsafe_scope()
        }
        self.metrics
            .counters
            .methods
            .count(i.sig.unsafety.is_some());
        visit::visit_impl_item_method(self, i);
        if i.sig.unsafety.is_some() {
            self.exit_unsafe_scope()
        }
    }

    // TODO: Visit macros.
    //
    // TODO: Figure out if there are other visit methods that should be
    // implemented here.
}
