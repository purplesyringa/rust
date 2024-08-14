use std::assert_matches::assert_matches;

use rustc_macros::{extension, HashStable, TyDecodable, TyEncodable, TypeFoldable, TypeVisitable};

use super::Const;
use crate::mir;
use crate::ty::abstract_const::CastKind;
use crate::ty::visit::TypeVisitableExt as _;
use crate::ty::{self, Ty, TyCtxt};

#[extension(pub(crate) trait UnevaluatedConstEvalExt<'tcx>)]
impl<'tcx> ty::UnevaluatedConst<'tcx> {
    /// FIXME(RalfJung): I cannot explain what this does or why it makes sense, but not doing this
    /// hurts performance.
    #[inline]
    fn prepare_for_eval(
        self,
        tcx: TyCtxt<'tcx>,
        param_env: ty::ParamEnv<'tcx>,
    ) -> (ty::ParamEnv<'tcx>, Self) {
        // HACK(eddyb) this erases lifetimes even though `const_eval_resolve`
        // also does later, but we want to do it before checking for
        // inference variables.
        // Note that we erase regions *before* calling `with_reveal_all_normalized`,
        // so that we don't try to invoke this query with
        // any region variables.

        // HACK(eddyb) when the query key would contain inference variables,
        // attempt using identity args and `ParamEnv` instead, that will succeed
        // when the expression doesn't depend on any parameters.
        // FIXME(eddyb, skinny121) pass `InferCtxt` into here when it's available, so that
        // we can call `infcx.const_eval_resolve` which handles inference variables.
        if (param_env, self).has_non_region_infer() {
            (
                tcx.param_env(self.def),
                ty::UnevaluatedConst {
                    def: self.def,
                    args: ty::GenericArgs::identity_for_item(tcx, self.def),
                },
            )
        } else {
            (tcx.erase_regions(param_env).with_reveal_all_normalized(tcx), tcx.erase_regions(self))
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[derive(HashStable, TyEncodable, TyDecodable, TypeVisitable, TypeFoldable)]
pub enum ExprKind {
    Binop(mir::BinOp),
    UnOp(mir::UnOp),
    FunctionCall,
    Cast(CastKind),
}
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[derive(HashStable, TyEncodable, TyDecodable, TypeVisitable, TypeFoldable)]
pub struct Expr<'tcx> {
    pub kind: ExprKind,
    args: ty::GenericArgsRef<'tcx>,
}

impl<'tcx> rustc_type_ir::inherent::ExprConst<TyCtxt<'tcx>> for Expr<'tcx> {
    fn args(self) -> ty::GenericArgsRef<'tcx> {
        self.args
    }
}

impl<'tcx> Expr<'tcx> {
    pub fn new_binop(
        tcx: TyCtxt<'tcx>,
        binop: mir::BinOp,
        lhs_ty: Ty<'tcx>,
        rhs_ty: Ty<'tcx>,
        lhs_ct: Const<'tcx>,
        rhs_ct: Const<'tcx>,
    ) -> Self {
        let args = tcx.mk_args_from_iter::<_, ty::GenericArg<'tcx>>(
            [lhs_ty.into(), rhs_ty.into(), lhs_ct.into(), rhs_ct.into()].into_iter(),
        );

        Self { kind: ExprKind::Binop(binop), args }
    }

    pub fn binop_args(self) -> (Ty<'tcx>, Ty<'tcx>, Const<'tcx>, Const<'tcx>) {
        assert_matches!(self.kind, ExprKind::Binop(_));

        match self.args().as_slice() {
            [lhs_ty, rhs_ty, lhs_ct, rhs_ct] => (
                lhs_ty.expect_ty(),
                rhs_ty.expect_ty(),
                lhs_ct.expect_const(),
                rhs_ct.expect_const(),
            ),
            _ => bug!("Invalid args for `Binop` expr {self:?}"),
        }
    }

    pub fn new_unop(tcx: TyCtxt<'tcx>, unop: mir::UnOp, ty: Ty<'tcx>, ct: Const<'tcx>) -> Self {
        let args =
            tcx.mk_args_from_iter::<_, ty::GenericArg<'tcx>>([ty.into(), ct.into()].into_iter());

        Self { kind: ExprKind::UnOp(unop), args }
    }

    pub fn unop_args(self) -> (Ty<'tcx>, Const<'tcx>) {
        assert_matches!(self.kind, ExprKind::UnOp(_));

        match self.args().as_slice() {
            [ty, ct] => (ty.expect_ty(), ct.expect_const()),
            _ => bug!("Invalid args for `UnOp` expr {self:?}"),
        }
    }

    pub fn new_call(
        tcx: TyCtxt<'tcx>,
        func_ty: Ty<'tcx>,
        func_expr: Const<'tcx>,
        arguments: impl IntoIterator<Item = Const<'tcx>>,
    ) -> Self {
        let args = tcx.mk_args_from_iter::<_, ty::GenericArg<'tcx>>(
            [func_ty.into(), func_expr.into()]
                .into_iter()
                .chain(arguments.into_iter().map(|ct| ct.into())),
        );

        Self { kind: ExprKind::FunctionCall, args }
    }

    pub fn call_args(self) -> (Ty<'tcx>, Const<'tcx>, impl Iterator<Item = Const<'tcx>>) {
        assert_matches!(self.kind, ExprKind::FunctionCall);

        match self.args().as_slice() {
            [func_ty, func, rest @ ..] => (
                func_ty.expect_ty(),
                func.expect_const(),
                rest.iter().map(|arg| arg.expect_const()),
            ),
            _ => bug!("Invalid args for `Call` expr {self:?}"),
        }
    }

    pub fn new_cast(
        tcx: TyCtxt<'tcx>,
        cast: CastKind,
        value_ty: Ty<'tcx>,
        value: Const<'tcx>,
        to_ty: Ty<'tcx>,
    ) -> Self {
        let args = tcx.mk_args_from_iter::<_, ty::GenericArg<'tcx>>(
            [value_ty.into(), value.into(), to_ty.into()].into_iter(),
        );

        Self { kind: ExprKind::Cast(cast), args }
    }

    pub fn cast_args(self) -> (Ty<'tcx>, Const<'tcx>, Ty<'tcx>) {
        assert_matches!(self.kind, ExprKind::Cast(_));

        match self.args().as_slice() {
            [value_ty, value, to_ty] => {
                (value_ty.expect_ty(), value.expect_const(), to_ty.expect_ty())
            }
            _ => bug!("Invalid args for `Cast` expr {self:?}"),
        }
    }

    pub fn new(kind: ExprKind, args: ty::GenericArgsRef<'tcx>) -> Self {
        Self { kind, args }
    }

    pub fn args(self) -> ty::GenericArgsRef<'tcx> {
        self.args
    }
}

#[cfg(target_pointer_width = "64")]
rustc_data_structures::static_assert_size!(Expr<'_>, 16);
