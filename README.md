[![Build Status](https://travis-ci.org/TyanNN/libtabun.rs.svg?branch=master)](https://travis-ci.org/TyanNN/libtabun.rs)
[![GPL Licence](https://badges.frapsoft.com/os/gpl/gpl.svg?v=103)](https://opensource.org/licenses/GPL-2.0/)  
# libtabun.rs
API для tabun.everypony.ru
<sub>Точно работает на nightly версии rust, насчёт стабильной не знаю, смотрите на Travis</sub>

# Установка

```toml
[dependencies]
libtabun = { git = "https://github.com/TyanNN/libtabun.rs" }
```

```bash
cargo build
```
# Документация

Можно почитать [тут](https://kotobank.ch/~easy/libtabun/doc/libtabun/) или собрать самому:

```bash
cargo doc
```

# Roadmap
- [x] Логин
- [x] Читать
  - [x] Посты
  - [x] Комменты
  - [x] Личные сообщения
  - [x] Инфу о юзерах
- [x] Создавать
  - [x] Комменты
	- [x] В личных сообщениях
  - [x] Посты
    - [x] Редактирование
    - [x] Удаление
  - [x] Личные сообщения
- [x] Опросы
- [x] Инвайты
- [x] Избранное
  - [x] Посты
  - [x] Комменты
- [x] Активность из /comments
- [x] Загрузка картинок по ссылке
