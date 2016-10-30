/* Main library file
 *
 * Copyright (C) 2016 TyanNN <TyanNN@cocaine.ninja>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

//! Эта библиотека предназначена для
//! взаимодействия с [табуном](https://tabun.everypony.ru)
//! (и потенциально прочими сайтами на лайвстрите), так как
//! API у них нет.
//!
//! Весь интерфейс находится в [`TClient`](struct.TClient.html), хотя на самом деле
//! разнесён по нескольким файлам.
//!
//! Большинство функций ~~нагло украдены~~ портированы с [`tabun_api`](https://github.com/andreymal/tabun_api)
//!
//! # Examples
//!
//! ```no_run
//! let mut user = libtabun::TClient::new("username", "password").unwrap();
//! let posts = user.get_posts("fanart", 1).unwrap();
//! for post in &posts {
//!     println!("{} - {}", post.id, post.title);
//! }
//! ```
//!
//! Можно использовать [`TClientBuilder`](struct.TClientBuilder.html) для
//! большей кастомизации:
//!
//! ```no_run
//! let mut user = libtabun::TClientBuilder::new()
//!     .session_id("t15sacuhntote9h99190vtne1m")
//!     .key("329tZL5OoJRvw6SxcmLpfMFXCt7mfjcU")
//!     .host("http://tabun-dev.localhost")
//!     .finalize().unwrap();
//! println!(
//!     "Logged in as {}",
//!     if user.name.is_empty() { "[none]" } else { user.name.as_str() }
//! );
//! ```

extern crate hyper;
extern crate select;
extern crate regex;
extern crate url;
extern crate multipart;
extern crate unescape;
extern crate serde_json;
#[macro_use] extern crate hado;

use std::fmt::Display;
use std::str::FromStr;

use regex::Regex;

use std::collections::HashMap;

use hyper::client::Client;
use hyper::client::request::Request;
use hyper::header::{CookiePair,CookieJar,SetCookie,Cookie};
use hyper::status::StatusCode;

use multipart::client::Multipart;

use std::io::Read;

use select::document::Document;
use select::predicate::{Class, Name, And, Attr};

use serde_json::Value;

#[macro_use] mod utils;
mod comments;
mod posts;
mod talks;

//Перечисления

#[derive(Debug)]
pub enum TabunError {
    ///На случай `Hacking attempt!`
    HackingAttempt,

    ///Ошибка с названием и описанием,
    ///обычно соответствует табуновским
    ///всплывающим сообщениям
    Error(String,String),

    ///Ошибка с номером, вроде 404 и 403
    NumError(StatusCode),

    ///Ошибка HTTP или ошибка сети, которая может быть при плохом интернете
    ///или лежачем Табуне
    IoError(hyper::error::Error),

    ///Ошибка парсинга страницы. Скорее всего будет возникать после изменения
    ///вёрстки Табуна, поэтому имеет смысл сообщать об этой ошибке
    ///разработчикам
    ParseError(String, u32, String)
}

///Тип комментария для ответа
pub enum CommentType {
    ///Комментарий к посту
    Post,

    ///Ответ на личное сообщение
    Talk
}

//Структуры

///Клиент табуна
pub struct TClient<'a> {
    pub name:               String,
    pub security_ls_key:    String,
    pub host:               String,
    client:                 Client,
    cookies:                CookieJar<'a>,
}

///Строитель клиента табуна
pub struct TClientBuilder {
    login:            String,
    pass:             String,
    session_id:       String,
    security_ls_key:  String,
    key:              String,
    client:           Client,
    host:             String,
    session_id_name:  String
}

#[derive(Debug,Clone)]
pub struct Comment {
    pub body:       String,
    pub id:         u32,
    pub author:     String,
    pub date:       String,
    pub votes:      i32,
    pub parent:     u32,
    pub post_id:    u32,
    pub deleted:    bool
}

#[derive(Debug,Clone)]
pub struct Post {
    pub title:          String,
    pub body:           String,
    pub date:           String,
    pub tags:           Vec<String>,
    pub comments_count: u32,
    pub author:         String,
    pub id:             u32,
}

#[derive(Debug,Clone)]
pub struct EditablePost {
    pub title:          String,
    pub body:           String,
    pub tags:           Vec<String>,
}

///Блоги из списка блогов в [профиле](struct.UserInfo.html)
#[derive(Debug,Clone)]
pub struct InBlogs {
    ///Созданные пользователем блоги
    pub created: Vec<String>,

    ///Блоги, в которых пользователь является администратором
    pub admin: Vec<String>,

    ///Блоги, в которых пользователь является модератором
    pub moderator: Vec<String>,

    ///Блоги, в которых пользователь состоит
    pub member: Vec<String>
}


///Профиль некоторого пользователя
#[derive(Debug,Clone)]
pub struct UserInfo {
    pub username:       String,
    pub realname:       String,

    ///Силушка
    pub skill:          f32,
    pub id:             u32,

    ///Кармочка
    pub rating:         f32,

    ///URL картинки, иногда с `//`, иногда с `https://`
    pub userpic:        String,
    pub description:    String,

    ///Информация вроде даты рождения и последнего визита,
    ///поля называются как на сайте
    pub other_info:     HashMap<String,String>,

    ///Блоги, которые юзер создал/состоит в них/модерирует
    pub blogs:          InBlogs,

    ///Кол-во публикаций
    pub publications:   u32,

    ///Кол-во избранного
    pub favourites:     u32,

    ///Кол-во друзей
    pub friends:        u32
}

///Диалог в личных сообщениях
#[derive(Debug,Clone)]
pub struct Talk {
    pub title:  String,
    pub body:   String,

    ///Участники
    pub users:  Vec<String>,
    pub comments: HashMap<u32, Comment>,
    pub date:   String
}

///Список личных сообщений
#[derive(Debug,Clone)]
pub struct TalkItem {
    pub id: u32,
    pub title:  String,
    pub users:  Vec<String>,
}

//Реализации

impl From<StatusCode> for TabunError {
    fn from(x: StatusCode) -> Self {
        TabunError::NumError(x)
    }
}

impl From<hyper::error::Error> for TabunError {
    fn from(x: hyper::error::Error) -> Self {
        TabunError::IoError(x)
    }
}

impl From<std::io::Error> for TabunError {
    fn from(x: std::io::Error) -> Self {
        TabunError::IoError(hyper::Error::Io(x))
    }
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Comment({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
}

impl Display for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Post({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
}

impl Display for UserInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "UserInfo({},\"{}\",\"{}\")", self.username, self.skill, self.rating)
    }
}

///URL сайта. Ибо по идее может работать и с другими штуками на лайвстрите
pub const HOST_URL: &'static str = "https://tabun.everypony.ru";

pub type TabunResult<T> = Result<T,TabunError>;

impl TClientBuilder {
    pub fn new() -> TClientBuilder {
        TClientBuilder {
            login:            String::new(),
            pass:             String::new(),
            session_id:       String::new(),
            security_ls_key:  String::new(),
            key:              String::new(),
            client:           Client::new(),
            host:             HOST_URL.to_string(),
            session_id_name:  String::from("TABUNSESSIONID"),
        }
    }

    pub fn login(mut self, login: &str) -> TClientBuilder {
        self.login = login.to_string();
        self
    }

    pub fn pass(mut self, pass: &str) -> TClientBuilder {
        self.pass = pass.to_string();
        self
    }

    pub fn session_id(mut self, session_id: &str) -> TClientBuilder {
        self.session_id = session_id.to_string();
        self
    }

    pub fn security_ls_key(mut self, security_ls_key: &str) -> TClientBuilder {
        self.security_ls_key = security_ls_key.to_string();
        self
    }

    pub fn key(mut self, key: &str) -> TClientBuilder {
        self.key = key.to_string();
        self
    }

    pub fn host(mut self, host: &str) -> TClientBuilder {
        self.host = host.to_string();
        self
    }

    pub fn session_id_name(mut self, session_id_name: &str) -> TClientBuilder {
        self.session_id_name = session_id_name.to_string();
        self
    }

    pub fn client(mut self, client: Client) -> TClientBuilder {
        self.client = client;
        self
    }

    pub fn finalize<'a>(self) -> TabunResult<TClient<'a>> {
        let mut user = TClient{
            name:               String::new(),
            security_ls_key:    self.security_ls_key.clone(),
            client:             self.client,
            cookies:            CookieJar::new(format!("{:?}",std::time::SystemTime::now()).as_bytes()),
            host:               self.host,
        };

        // Проставляем печеньки какие есть
        if !self.session_id.is_empty() {
            user.cookies.add(CookiePair::new(self.session_id_name, self.session_id));
        }

        if !self.key.is_empty() {
            user.cookies.add(CookiePair::new("key".to_string(), self.key));
        }

        // Качаем главную страницу, попутно это проставит отсутствующие печеньки
        let data = try!(user.get_bytes("/"));
        // Парсим информацию о текущем пользователе
        user.update_userinfo(&data);

        // Если текущего пользователя нет, но у нас есть логин и пароль, то логинимся
        if user.name.is_empty() && !self.login.is_empty() && !self.pass.is_empty() {
            try!(user.login(&self.login, &self.pass));
        }
        Ok(user)
    }
}

impl<'a> TClient<'a> {

    ///Входит на табунчик и сохраняет LIVESTREET_SECURITY_KEY,
    ///если логин или пароль == None - анонимус.
    ///
    ///# Examples
    ///```no_run
    ///let mut user = libtabun::TClient::new("логин","пароль");
    ///```
    pub fn new<T: Into<Option<&'a str>>>(login: T, pass: T) -> TabunResult<TClient<'a>> {
        let mut user = TClient{
            name:               String::new(),
            security_ls_key:    String::new(),
            client:             Client::new(),
            cookies:            CookieJar::new(format!("{:?}",std::time::SystemTime::now()).as_bytes()),
            host:               String::from(HOST_URL),
        };

        let data = try!(user.get_bytes("/"));
        user.update_userinfo(&data);

        if let (Some(login), Some(pass)) = (login.into(), pass.into()) {
            try!(user.login(login, pass));
        }

        Ok(user)
    }

    /// Парсит код страницы, переданной в параметре, и обновляет:
    /// - security_ls_key
    /// - имя пользователя
    fn update_userinfo(&mut self, data: &Vec<u8>) {
        let str_data = String::from_utf8_lossy(data).into_owned();
        let page = Document::from(str_data.as_str());

        // Ищем security_ls_key
        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();
        match ls_key_regex.captures(&str_data) {
            Some(x) => {
                self.security_ls_key = x.at(1).unwrap().to_owned();
            },
            None => {}
        };

        // Ищем панельку с информацией о текущем пользователе
        let dropdown_user = match page.find(Attr("id", "dropdown-user")).first() {
            None => {
                // Не нашли — значит скорее всего не залогинены
                self.name = String::new();
                return;
            },
            Some(x) => x,
        };

        self.name = match dropdown_user.find(Class("username")).first() {
            Some(x) => x.text(),
            None => String::new(),
        };
    }

    ///Заметка себе: создаёт промежуточный объект запроса, сразу выставляя печеньки,
    ///на случай если надо что-то поменять (как в delete_post)
    fn create_middle_req(&mut self, url: &str) -> hyper::client::RequestBuilder {
        let full_url = format!("{}{}", self.host, url); //TODO: Заменить на concat_idents! когда он стабилизируется
        self.client.get(&full_url)
            .header(Cookie::from_cookie_jar(&self.cookies))
    }

    fn get(&mut self,url: &str) -> TabunResult<Document> {
        let buf = String::from_utf8_lossy(
            &try!(self.get_bytes(url))
        ).into_owned();

        Ok(Document::from(&*buf))
    }

    fn get_bytes(&mut self, url: &str) -> TabunResult<Vec<u8>> {
        let mut res = try!(self.create_middle_req(url).send());

        if res.status != hyper::Ok {
            return Err(TabunError::from(res.status));
        }

        let mut buf: Vec<u8> = Vec::new();
        try!(res.read_to_end(&mut buf));

        if let Some(x) = res.headers.get::<SetCookie>() {
            x.apply_to_cookie_jar(&mut self.cookies);
        }

        Ok(buf)
    }

    fn multipart(&mut self,url: &str, bd: HashMap<&str,&str>) -> Result<hyper::client::Response, TabunError> {
        let url = format!("{}{}", self.host, url); //TODO: Заменить на concat_idents! когда он стабилизируется
        let mut request = Request::new(
            hyper::method::Method::Post,
            hyper::Url::from_str(&url).unwrap()
        ).unwrap();  // TODO: обработать нормально?
        request.headers_mut().set(Cookie::from_cookie_jar(&self.cookies));

        let mut req = Multipart::from_request(request).unwrap();

        for (param,val) in bd {
            let _ = req.write_text(param,val);
        }

        let res = try!(req.send());

        if let Some(x) = res.headers.get::<SetCookie>() {
            x.apply_to_cookie_jar(&mut self.cookies);
        }

        if res.status != hyper::Ok && res.status != hyper::status::StatusCode::MovedPermanently {
            return Err(TabunError::from(res.status));
        }

        Ok(res)
    }

    /// Отправляет ajax-запрос и возвращает распарсенный json-ответ (Value).
    /// Он гарантированно является json-объектом (то есть можно использовать
    /// `.as_object().unwrap()`, если нужно)
    fn ajax(&mut self, url: &str, bd: HashMap<&str, &str>) -> TabunResult<Value> {
        let key = self.security_ls_key.to_owned();

        let mut bd_ready = map!["security_ls_key" => key.as_str()];
        for (k, v) in &bd {
            bd_ready.insert(k, v);
        }

        let mut res = try!(self.multipart(url, bd_ready));

        let mut data = String::new();
        try!(res.read_to_string(&mut data));
        let raw_data = data.trim();

        // Если накосячили с security_ls_key, то может прийти такая ошибка
        if raw_data.starts_with("Hacking") {
            return Err(TabunError::HackingAttempt);
        }

        let mut data = String::new();

        if raw_data.starts_with("<textarea>{") {
            // Иногда Табун зачем-то возвращает json-объект, обёрнутый в textarea
            let doc = Document::from(raw_data);
            let tmp = try_to_parse!(
                doc.find(Name("textarea")).first()
            ).text();
            data.push_str(tmp.as_str());

        } else {
            data.push_str(raw_data);
        }

        let data = try_to_parse_json!(data.as_str());

        if get_json!(data, "/bStateError", as_bool, false) {
            return Err(TabunError::Error(
                get_json!(data, "/sMsgTitle", as_str, "").to_string(),
                get_json!(data, "/sMsg", as_str, "").to_string(),
            ));
        }

        Ok(data)
    }

    ///Логинится с указанными именем пользователя и паролем
    pub fn login(&mut self, login: &str, pass: &str) -> TabunResult<()> {
        let host = self.host.to_owned();
        try!(self.ajax(
            "/login/ajax-login",
            map![
                "login" => login,
                "password" => pass,
                "return-path" => &host,
                "remember" => "on"
            ]
        ));

        self.name = String::from(login);
        Ok(())
    }

    ///Загружает картинку по URL, попутно вычищая табуновские бэкслэши из ответа
    pub fn upload_image_from_url(&mut self, url: &str) -> TabunResult<String> {
        let data = try!(self.ajax(
            "/ajax/upload/image",
            map![
                "title" => "",
                "img_url" => url
            ]
        ));

        let text = get_json!(data, "/sText", as_str);
        let text = match text {
            Some(x) => x,
            None => return Err(parse_error!("Server did not return sText"))
        };

        let doc = Document::from(text);
        let img = try_to_parse!(doc.find(Name("img")).first());
        Ok(try_to_parse!(img.attr("src")).to_string())
    }

    ///Получает ID блога по его имени
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("lighthouse").unwrap();
    ///assert_eq!(blog_id,15558);
    ///```
    pub fn get_blog_id(&mut self,name: &str) -> TabunResult<u32> {
        let url = format!("/blog/{}", name);
        let page = try!(self.get(&url));

        Ok(try_to_parse!(hado!{
            el <- page.find(And(Name("div"),Class("vote-item"))).find(Name("span")).first();
            id_s <- el.attr("id");
            num_s <- id_s.split('_').last();
            num_s.parse::<u32>().ok()
        }))
    }

    ///Получает инфу о пользователе,
    ///если указан как None, то получает инфу о
    ///текущем пользователе
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_profile("Orhideous");
    pub fn get_profile<'f, T: Into<Option<&'f str>>>(&mut self, name: T) -> TabunResult<UserInfo> {
        let name = match name.into() {
            Some(x) => x.to_owned(),
            None    => self.name.to_owned()
        };

        let full_url = format!("/profile/{}", name);
        let page = try!(self.get(&full_url));
        let profile = page.find(And(Name("div"),Class("profile")));

        let username = try_to_parse!(
                profile.find(And(Name("h2"),Attr("itemprop","nickname"))).first()
            ).text();

        let realname = match profile.find(And(Name("p"),Attr("itemprop","name"))).first() {
                Some(x) => x.text(),
                None => String::new()
        };

        let (skill,user_id) = try_to_parse!(hado!{
            skill_area <- profile.find(And(Name("div"),Class("strength"))).find(Name("div")).first();
            skill <- skill_area.text().parse::<f32>().ok();
            user_id <- hado!{
                id_s <- skill_area.attr("id");
                elm <- id_s.split('_').collect::<Vec<_>>().get(2);
                elm.parse::<u32>().ok()
            };
            Some((skill,user_id))
        });

        let rating = try_to_parse!(hado!{
            el <- profile.find(Class("vote-count")).find(Name("span")).first();
            el.text().parse::<f32>().ok()
        });

        let about = try_to_parse!(page.find(And(Name("div"),Class("profile-info-about"))).first());

        let userpic = try_to_parse!(about.find(Class("avatar")).find(Name("img")).first());
        let userpic = try_to_parse!(userpic.attr("src"));

        let description = try_to_parse!(about.find(And(Name("div"),Class("text"))).first()).inner_html();

        let dotted = page.find(And(Name("ul"), Class("profile-dotted-list")));
        let dotted = try_to_parse!(dotted.iter().last()).find(Name("li"));

        let mut other_info = HashMap::<String,String>::new();

        let mut created = Vec::<String>::new();
        let mut admin = Vec::<String>::new();
        let mut moderator = Vec::<String>::new();
        let mut member= Vec::<String>::new();

        for li in dotted.iter() {
            let name = try_to_parse!(li.find(Name("span")).first()).text();
            let val = try_to_parse!(li.find(Name("strong")).first());

            if name.contains("Создал"){
                created = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else if name.contains("Администрирует") {
                admin = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else if name.contains("Модерирует") {
                moderator = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else if name.contains("Состоит") {
                member = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else {
                other_info.insert(name.replace(":",""),val.text());
            }
        }

        let blogs = InBlogs{
            created: created,
            admin: admin,
            moderator: moderator,
            member: member
        };

        let nav = page.find(Class("nav-profile")).find(Name("li"));

        let (mut publications,mut favourites, mut friends) = (0,0,0);

        for li in nav.iter() {
            let a = try_to_parse!(li.find(Name("a")).first()).text();

            if !a.contains("Инфо") {
                 let a = a.split('(').collect::<Vec<_>>();
                 if a.len() >1 {
                     let val = try_to_parse!(a[1].replace(")","")
                         .parse::<u32>().ok());
                     if a[0].contains(&"Публикации") {
                         publications = val
                     } else if a[0].contains(&"Избранное") {
                         favourites = val
                     } else {
                         friends = val
                     }
                 }
            }
        }

        Ok(UserInfo{
            username:       username,
            realname:       realname,
            skill:          skill,
            id:             user_id,
            rating:         rating,
            userpic:        userpic.to_owned(),
            description:    description,
            other_info:     other_info,
            blogs:          blogs,
            publications:   publications,
            favourites:     favourites,
            friends:        friends
        })
    }

    ///Добавляет что-то в избранное, true - коммент, false - пост
    ///(внутренний метод для публичных favourite_post и favourite_comment)
    fn favourite(&mut self, id: u32, typ: bool, fn_typ: bool) -> TabunResult<u32> {
        let id = id.to_string();

        let body = map![
            if fn_typ { "idComment"} else { "idTopic" } => id.as_str(),
            "type" => &(if typ { "1" } else { "0" })
        ];

        let data = try!(self.ajax(
            &format!("/ajax/favourite/{}/", if fn_typ { "comment" } else { "topic" }),
            body
        ));

        match get_json!(data, "/iCount", as_u64) {
            Some(fav_cnt) => Ok(fav_cnt as u32),
            None => Err(parse_error!("Server did not return iCount"))
        }
    }
}

#[cfg(test)]
mod test {
    use ::{TClient};
    use ::regex::{Error,Regex};

    #[test]
    fn test_parsetext_macro() {
        let r : Result<u32, Error> = parse_text_to_res!(regex => r"sometext (\d+) sometext", st => "sometext 001 sometext", num => 1, typ => u32);
        match r {
            Ok(x)   => assert_eq!(x, 1),
            Err(_)  => unreachable!()
        }
    }

    #[test]
    fn test_blog_id() {
        let mut user = TClient::new(None,None).unwrap();
        match user.get_blog_id("herp_derp") {
            Ok(x)   => assert_eq!(193, x),
            Err(x)  => panic!(x)
        }
    }

    #[test]
    fn test_get_profile() {
        let mut user = TClient::new(None,None).unwrap();
        match user.get_profile("OrHiDeOuS") {
            Ok(x)   => assert_eq!(x.username, "Orhideous"),
            Err(x)  => panic!(x)
        }
    }
}
