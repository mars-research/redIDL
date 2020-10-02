// Compiler requires that all paramter types implement this marker
pub trait Exchangeable {}

// Make proxy references exchangeable
impl<T: crate::Proxy + ?Sized> Exchangeable for &T {}
impl<T: crate::Proxy + ?Sized> Exchangeable for &mut T {}

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

impl<T: Exchangeable, const N: usize> Exchangeable for [T; N] {}
impl<T: Exchangeable> Exchangeable for [T] {}

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
