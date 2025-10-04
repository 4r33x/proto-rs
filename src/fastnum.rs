mod signed;
mod unsigned;

pub use signed::D128Proto;
pub use unsigned::UD128Proto;

#[cfg(test)]
mod tests {
    use fastnum::UD128;

    #[test]
    fn test_u128() {
        UD128::from_u128(u128::MAX).unwrap();
    }
}
