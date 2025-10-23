use tabulon::{CompiledExpr, JitError, Tabula, Parser, IdentityResolver, VarAccessStrategy};
use tabulon::engine::CtxFamily;

struct LifeTimeCtxFamily;

impl CtxFamily for LifeTimeCtxFamily {
    type Ctx<'a> = LifetimeCtx<'a>;
}

// 1. Define a context struct with a lifetime.
#[repr(C)]
struct LifetimeCtx<'a> {
    value: &'a f64,
}

// 2. Define an extern "C" shim function for the resolver.
// This bypasses the complexities of the #[resolver] macro with lifetimes.
extern "C" fn my_resolver_shim(ctx: *mut std::ffi::c_void, _index: u32) -> f64 {
    if ctx.is_null() {
        return f64::NAN;
    }
    // Unsafe is necessary here, but the overall safety is guaranteed by the
    // Tabula<Ctx> type parameter ensuring only the correct Ctx type is ever used.
    let c = unsafe { &*(ctx as *const LifetimeCtx) };
    *c.value
}

// A non-generic wrapper for resolver-based evaluation
struct SpecificResolverWrapper {
    expr: CompiledExpr<String, LifeTimeCtxFamily>,
    name: String,
}

impl SpecificResolverWrapper {
    // This eval method works with a LifetimeCtx of *any* lifetime 'a.
    pub fn eval<'a>(&self, ctx: &mut LifetimeCtx<'a>) -> Result<f64, JitError> {
        self.expr.eval_resolver_ctx(ctx)
    }
}

// 3. Helper function to create a reusable SpecificResolverWrapper.
fn prepare_expr_resolver_wrapped() -> SpecificResolverWrapper {
    let mut eng = Tabula::<LifeTimeCtxFamily>::new_ctx();

    // Register the resolver shim function directly.
    let resolver_symbol = "my_resolver_shim";
    eng.set_var_getter(resolver_symbol, my_resolver_shim).unwrap();

    // Parse an expression with a variable that the resolver will provide.
    let parser = Parser::new("my_var").unwrap();
    let prepared = parser.parse_with_var_resolver(&IdentityResolver).unwrap();

    // Compile with the explicit resolver strategy.
    let expr = eng.compile_prepared_with(&prepared, VarAccessStrategy::ResolverCall { symbol: resolver_symbol }).unwrap();

    SpecificResolverWrapper {
        expr,
        name: "specific_resolver_wrapper".to_string(),
    }
}

#[test]
fn test_reusable_resolver_wrapper_with_lifetimed_ctx() {
    // 4. Get the reusable, 'static wrapped expression.
    let wrapped_expr = prepare_expr_resolver_wrapped();
    assert_eq!(wrapped_expr.name, "specific_resolver_wrapper");

    // 5. Create a new scope to have a shorter lifetime 'a.
    {
        let local_data = 42.0;
        let mut short_lived_ctx = LifetimeCtx { value: &local_data };

        // 6. Evaluate the expression via the wrapper.
        let result = wrapped_expr.eval(&mut short_lived_ctx).unwrap();

        assert_eq!(result, 42.0);
    }

    // Another scope to prove reusability
    {
        let another_local_data = 100.0;
        let mut another_short_lived_ctx = LifetimeCtx { value: &another_local_data };
        let result = wrapped_expr.eval(&mut another_short_lived_ctx).unwrap();
        assert_eq!(result, 100.0);
    }
}