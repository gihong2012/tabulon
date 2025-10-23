use tabulon::{CompiledExpr, JitError, Tabula};

// 1. Define a context struct with a lifetime.
#[repr(C)]
struct LifetimeCtx<'a> {
    value: &'a f64,
}

// A non-generic wrapper with fixed K=String and Ctx=LifetimeCtx<'static>
struct SpecificWrapper {
    expr: CompiledExpr<String, LifetimeCtx<'static>>,
    name: String,
}

#[derive(Debug)]
enum SomeErr {
    LibErr(JitError)
}

impl SpecificWrapper {
    // This eval method works with a LifetimeCtx of *any* lifetime 'a.
    pub fn eval<'a>(&self, values: &[f64], ctx: &mut LifetimeCtx<'a>) -> Result<f64, SomeErr> {
        self.expr.eval_with_ctx(values, ctx).map_err(|e| SomeErr::LibErr(e))
    }
}

// 2. Define an extern "C" function that uses this context.
extern "C" fn get_value(ctx: *mut std::ffi::c_void) -> f64 {
    if ctx.is_null() {
        return f64::NAN;
    }
    // Unsafe block to dereference the raw pointer and access the context.
    let c = unsafe { &*(ctx as *const LifetimeCtx) };
    *c.value
}

// 3. Helper function to create a reusable SpecificWrapper.
fn prepare_expr_wrapped() -> SpecificWrapper {
    let mut eng = Tabula::<LifetimeCtx<'static>>::new_ctx();
    eng.register_nullary("get_value", get_value, true).unwrap();
    let expr = eng.compile("get_value()").unwrap();
    SpecificWrapper {
        expr,
        name: "specific_wrapped_get_value".to_string(),
    }
}

#[test]
fn test_reusable_specific_wrapper_with_lifetimed_ctx() {
    // 4. Get the reusable, 'static wrapped expression.
    let wrapped_expr = prepare_expr_wrapped();
    assert_eq!(wrapped_expr.name, "specific_wrapped_get_value");

    // 5. Create a new scope to have a shorter lifetime 'a.
    {
        let local_data = 42.0;
        // Create a context with a lifetime 'a tied to this scope.
        let mut short_lived_ctx = LifetimeCtx { value: &local_data };

        // 6. Evaluate the 'static expression with the short-lived context via the wrapper.
        let result = wrapped_expr.eval(&[], &mut short_lived_ctx).unwrap();

        assert_eq!(result, 42.0);
    }

    // Another scope to prove reusability
    {
        let another_local_data = 100.0;
        let mut another_short_lived_ctx = LifetimeCtx { value: &another_local_data };
        let result = wrapped_expr.eval(&[], &mut another_short_lived_ctx).unwrap();
        assert_eq!(result, 100.0);
    }
}