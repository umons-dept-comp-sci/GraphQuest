use std::io::BufRead;

use gquest_core::data_handler::data_loader::GengProcess;

#[test]
fn load_geng_api() {
    let mut call = GengProcess::call_geng(10, &"".to_string(), (None, None)).expect("good");

    let f = call.get_reader();
    let mut count = 0;
    for s in f.lines().map_while(Result::ok) {
        // println!("Res: {s}");
        count += 1;
    }
    println!("{count}");

    call.wait_close();
}