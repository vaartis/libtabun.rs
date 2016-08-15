use ::{TClient,TabunError,Talk};

use select::predicate::{Class, Name, And};

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
}
