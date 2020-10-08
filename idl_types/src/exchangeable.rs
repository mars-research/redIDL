// Compiler requires that all parameter types implement this marker
pub trait Exchangeable {}
pub trait MemberExchangeable {}

/*
	References to Proxies
	Primitives
	RRef types
	anonymous composites of Copy types
*/

// Make proxy references exchangeable
impl<T> Exchangeable for &T where T: crate::Proxy + ?Sized {}
impl<T> Exchangeable for &mut T where T: crate::Proxy + ?Sized {}

impl Exchangeable for i8 {}
impl Exchangeable for i16 {}
impl Exchangeable for i32 {}
impl Exchangeable for i64 {}
impl Exchangeable for i128 {}
impl Exchangeable for isize {}
impl Exchangeable for u8 {}
impl Exchangeable for u16 {}
impl Exchangeable for u32 {}
impl Exchangeable for u64 {}
impl Exchangeable for u128 {}
impl Exchangeable for usize {}
impl Exchangeable for f32 {}
impl Exchangeable for f64 {}
impl Exchangeable for char {}
impl Exchangeable for bool {}

impl MemberExchangeable for i8 {}
impl MemberExchangeable for i16 {}
impl MemberExchangeable for i32 {}
impl MemberExchangeable for i64 {}
impl MemberExchangeable for i128 {}
impl MemberExchangeable for isize {}
impl MemberExchangeable for u8 {}
impl MemberExchangeable for u16 {}
impl MemberExchangeable for u32 {}
impl MemberExchangeable for u64 {}
impl MemberExchangeable for u128 {}
impl MemberExchangeable for usize {}
impl MemberExchangeable for f32 {}
impl MemberExchangeable for f64 {}
impl MemberExchangeable for char {}
impl MemberExchangeable for bool {}

impl<T: Exchangeable, const N: usize> Exchangeable for [T; N] {}
impl<T: Exchangeable> Exchangeable for [T] {}

impl<T: Exchangeable, const N: usize> MemberExchangeable for [T; N] {}
impl<T: Exchangeable> MemberExchangeable for [T] {}

impl<A: Exchangeable, B: Exchangeable> Exchangeable for (A, B) {}
impl<A: Exchangeable, B: Exchangeable, C: Exchangeable> Exchangeable for (A, B, C) {}
impl<A: Exchangeable, B: Exchangeable, C: Exchangeable, D: Exchangeable> Exchangeable
	for (A, B, C, D)
{
}

impl<A: Exchangeable, B: Exchangeable, C: Exchangeable, D: Exchangeable, E: Exchangeable>
	Exchangeable for (A, B, C, D, E)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
		H: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G, H)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
		H: Exchangeable,
		I: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G, H, I)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
		H: Exchangeable,
		I: Exchangeable,
		J: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G, H, I, J)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
		H: Exchangeable,
		I: Exchangeable,
		J: Exchangeable,
		K: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G, H, I, J, K)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
		H: Exchangeable,
		I: Exchangeable,
		J: Exchangeable,
		K: Exchangeable,
		L: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G, H, I, J, K, L)
{
}

impl<
		A: Exchangeable,
		B: Exchangeable,
		C: Exchangeable,
		D: Exchangeable,
		E: Exchangeable,
		F: Exchangeable,
		G: Exchangeable,
		H: Exchangeable,
		I: Exchangeable,
		J: Exchangeable,
		K: Exchangeable,
		L: Exchangeable,
		M: Exchangeable,
	> Exchangeable for (A, B, C, D, E, F, G, H, I, J, K, L, M)
{
}

impl<A: MemberExchangeable, B: MemberExchangeable> MemberExchangeable for (A, B) {}
impl<A: MemberExchangeable, B: MemberExchangeable, C: MemberExchangeable> MemberExchangeable for (A, B, C) {}
impl<A: MemberExchangeable, B: MemberExchangeable, C: MemberExchangeable, D: MemberExchangeable> MemberExchangeable
	for (A, B, C, D)
{
}

impl<A: MemberExchangeable, B: MemberExchangeable, C: MemberExchangeable, D: MemberExchangeable, E: MemberExchangeable>
	MemberExchangeable for (A, B, C, D, E)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
		H: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G, H)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
		H: MemberExchangeable,
		I: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G, H, I)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
		H: MemberExchangeable,
		I: MemberExchangeable,
		J: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G, H, I, J)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
		H: MemberExchangeable,
		I: MemberExchangeable,
		J: MemberExchangeable,
		K: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G, H, I, J, K)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
		H: MemberExchangeable,
		I: MemberExchangeable,
		J: MemberExchangeable,
		K: MemberExchangeable,
		L: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G, H, I, J, K, L)
{
}

impl<
		A: MemberExchangeable,
		B: MemberExchangeable,
		C: MemberExchangeable,
		D: MemberExchangeable,
		E: MemberExchangeable,
		F: MemberExchangeable,
		G: MemberExchangeable,
		H: MemberExchangeable,
		I: MemberExchangeable,
		J: MemberExchangeable,
		K: MemberExchangeable,
		L: MemberExchangeable,
		M: MemberExchangeable,
	> MemberExchangeable for (A, B, C, D, E, F, G, H, I, J, K, L, M)
{
}
