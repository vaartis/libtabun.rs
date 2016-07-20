extern crate libtabun;

use libtabun::TClient;

fn main() {
    let mut user = TClient::new();
    user.login("easyrainbow","flutteron");
    println!("{}",user.comment(157676, "Проверка реквестов раста[3]",11362196));
}
