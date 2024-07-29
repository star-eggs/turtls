use crate::big_int::UBigInt;
use crate::elliptic_curve::secp256r1::{FieldElement, Point};
pub fn generate_signature(
    msg: &[u8],
    key: FieldElement,
    hash_func: fn(&[u8]) -> [u8; 32],
    secret_num: FieldElement,
) -> (FieldElement, FieldElement) {
    let hash: FieldElement = UBigInt::<4>::from_be_bytes(hash_func(msg)).into();
    let inverse = secret_num.inverse();

    let new_point = Point::G.mul_scalar(secret_num);

    let s = inverse.mul(&(hash.add(&(new_point.0.mul(&key)))));

    // TODO: destroy inverse

    (new_point.0, s)
}
