use std::io::BufRead;

use gquest_core::data_handler::data_loader::GengProcess;

#[test]
fn test_geng_call() {
    let call = GengProcess::call_geng(3, &"".to_string(), (None, None)).expect("good");
    let signatures = ["B?", "BO", "BW", "Bw"].to_vec();
    let f = call.get_reader();

    for (i, s) in f.lines().map_while(Result::ok).enumerate() {
        assert_eq!(s, signatures[i]);
    }
}
