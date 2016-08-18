extern crate regex;
extern crate hyper;

use ::{TClient,TabunError,Talk,HOST_URL};

use select::predicate::{Class, Name, And};

use std::str;

use regex::Regex;

use hyper::header::Referer;

impl<'a> TClient<'a> {

    ///Получает личный диалог по его ID
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_talk(123);
    pub fn get_talk(&mut self, talk_id: i32) -> Result<Talk,TabunError>{
        let url = format!("/talk/read/{}", talk_id);
        let page = try!(self.get(&url));

        let title = page.find(Class("topic-title"))
            .first()
            .unwrap()
            .text();

        let body = page.find(Class("topic-content"))
            .first()
            .unwrap()
            .inner_html();
        let body = body.trim().to_string();

        let date = page.find(And(Name("li"),Class("topic-info-date")))
            .find(Name("time"))
            .first()
            .unwrap();
        let date = date.attr("datetime")
            .unwrap()
            .to_string();

        let comments = try!(self.get_comments(&url));

        let users = page.find(Class("talk-recipients-header"))
            .find(Name("a"))
            .iter()
            .filter(|x| x.attr("class").unwrap().contains("username"))
            .map(|x| x.text().to_string())
            .collect::<Vec<_>>();

        Ok(Talk{
            title:      title,
            body:       body,
            comments:   comments,
            users:      users,
            date:       date
        })
    }

    ///Создаёт личный диалог с пользователями
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.add_talk(vec!["человек1","человек2"], "Название", "Текст");
    pub fn add_talk(&mut self, users: &Vec<&str>, title: &str, body:&str) -> Result<i32,TabunError> {
        use mdo::option::bind;

        let users = users.iter().fold(String::new(),|mut acc, x| { acc.push_str(&format!("{}, ", *x)); acc });
        let key = self.security_ls_key.clone();

        let fields = map![
            "submit_talk_add" => "Отправить",
            "security_ls_key" => &key,
            "talk_users" => &users,
            "talk_title" => &title,
            "talk_text" => &body
        ];

        let res = try!(self.multipart("/talk/add",fields));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
                regex       =<< Regex::new(r"read/(\d+)/$").ok();
                captures    =<< regex.captures(r);
                r           =<< captures.at(1);
                ret r.parse::<i32>().ok()
               ).unwrap())
    }

    ///Удаляет цепочку сообщений, и, так как табун ничего не возаращет по этому поводу,
    ///выдаёт Ok(true) в случае удачи
    pub fn delete_talk(&mut self, talk_id: i32) -> Result<bool,TabunError> {
        let url = format!("/talk/delete/{}/?security_ls_key={}", talk_id ,&self.security_ls_key);
        match self.create_middle_req(&url)
            .header(Referer(format!("{}/talk/{}/", HOST_URL, talk_id)))
            .send().unwrap().status {
                hyper::Ok => Ok(true),
                x @ _ => Err(TabunError::NumError(x))
            }
    }
}
