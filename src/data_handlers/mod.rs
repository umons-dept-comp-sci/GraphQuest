pub mod geng_api;



/// Returns the input_str as a vector with all the contained signatures
pub fn to_signature_vector(input_str : &str) -> Vec<&str>
{
    let mut signature_vec: Vec<&str> = input_str
       .split("\n")
       .collect();
    signature_vec.pop();
    return signature_vec;
}