CREATE VIRTUAL TABLE memories_fts USING fts5(content,content='memories',content_rowid='rowid',tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
 INSERT INTO memories_fts(rowid,content) VALUES(new.rowid,new.content);
END;
CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
 INSERT INTO memories_fts(memories_fts,rowid,content) VALUES('delete',old.rowid,old.content);
END;
CREATE TRIGGER memories_au AFTER UPDATE ON memories BEGIN
 INSERT INTO memories_fts(memories_fts,rowid,content) VALUES('delete',old.rowid,old.content);
 INSERT INTO memories_fts(rowid,content) VALUES(new.rowid,new.content);
END;
