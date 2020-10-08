pub trait RRefable {}
pub trait ElementRRefable {} // Optional<RRef<>> is not this
pub trait FieldRRefable {} // but this

/*
	We cannot allow any sort of anonymous composites of RRef-based types
	i.e. tuples or arrays of ElementRRefable are RRefable
	named composites of FieldRRefable (including Optional<RRef<>>)
	are RRefable, references to Proxys are RRefable
*/

// Make proxy references exchangeable
impl<T: crate::Proxy + ?Sized> RRefable for &T {}
impl<T: crate::Proxy + ?Sized> RRefable for &mut T {}

impl RRefable for i8 {}
impl RRefable for i16 {}
impl RRefable for i32 {}
impl RRefable for i64 {}
impl RRefable for i128 {}
impl RRefable for isize {}
impl RRefable for u8 {}
impl RRefable for u16 {}
impl RRefable for u32 {}
impl RRefable for u64 {}
impl RRefable for u128 {}
impl RRefable for usize {}
impl RRefable for f32 {}
impl RRefable for f64 {}
impl RRefable for char {}
impl RRefable for bool {}

impl<T: RRefable, const N: usize> RRefable for [T; N] {}
impl<T: RRefable> RRefable for [T] {}

impl<A: RRefable, B: RRefable> RRefable for (A, B) {}
impl<A: RRefable, B: RRefable, C: RRefable> RRefable for (A, B, C) {}
impl<A: RRefable, B: RRefable, C: RRefable, D: RRefable> RRefable for (A, B, C, D) {}
impl<A: RRefable, B: RRefable, C: RRefable, D: RRefable, E: RRefable> RRefable for (A, B, C, D, E) {}

impl<A: RRefable, B: RRefable, C: RRefable, D: RRefable, E: RRefable, F: RRefable> RRefable
	for (A, B, C, D, E, F)
{
}

impl<A: RRefable, B: RRefable, C: RRefable, D: RRefable, E: RRefable, F: RRefable, G: RRefable>
	RRefable for (A, B, C, D, E, F, G)
{
}

impl<
		A: RRefable,
		B: RRefable,
		C: RRefable,
		D: RRefable,
		E: RRefable,
		F: RRefable,
		G: RRefable,
		H: RRefable,
	> RRefable for (A, B, C, D, E, F, G, H)
{
}

impl<
		A: RRefable,
		B: RRefable,
		C: RRefable,
		D: RRefable,
		E: RRefable,
		F: RRefable,
		G: RRefable,
		H: RRefable,
		I: RRefable,
	> RRefable for (A, B, C, D, E, F, G, H, I)
{
}

impl<
		A: RRefable,
		B: RRefable,
		C: RRefable,
		D: RRefable,
		E: RRefable,
		F: RRefable,
		G: RRefable,
		H: RRefable,
		I: RRefable,
		J: RRefable,
	> RRefable for (A, B, C, D, E, F, G, H, I, J)
{
}

impl<
		A: RRefable,
		B: RRefable,
		C: RRefable,
		D: RRefable,
		E: RRefable,
		F: RRefable,
		G: RRefable,
		H: RRefable,
		I: RRefable,
		J: RRefable,
		K: RRefable,
	> RRefable for (A, B, C, D, E, F, G, H, I, J, K)
{
}

impl<
		A: RRefable,
		B: RRefable,
		C: RRefable,
		D: RRefable,
		E: RRefable,
		F: RRefable,
		G: RRefable,
		H: RRefable,
		I: RRefable,
		J: RRefable,
		K: RRefable,
		L: RRefable,
	> RRefable for (A, B, C, D, E, F, G, H, I, J, K, L)
{
}

impl<
		A: RRefable,
		B: RRefable,
		C: RRefable,
		D: RRefable,
		E: RRefable,
		F: RRefable,
		G: RRefable,
		H: RRefable,
		I: RRefable,
		J: RRefable,
		K: RRefable,
		L: RRefable,
		M: RRefable,
	> RRefable for (A, B, C, D, E, F, G, H, I, J, K, L, M)
{
}
