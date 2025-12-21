egui_pm

- Unzip the file, which will create a directory with the application binary and supporting files.
- It will create a SQLite database \_pmdb.db that will store all of the credentials.
- It will also need a local .env file that the user will need to create, and they will need
  to seed it with the AES IV Key (32 characters) that they choose. This can be any string.
- .env should have this key as AES_KEY.
- Errors should prompt towards resolutions.
