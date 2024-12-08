pub mod file_caster;
pub mod grpc_caster;
pub mod input_handler;
pub mod stdout_caster;
pub mod tcp_caster;
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
