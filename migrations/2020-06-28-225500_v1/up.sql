ALTER TABLE monitors ADD COLUMN initial_name VARCHAR(255) NULL;
UPDATE monitors m SET initial_name=(SELECT name FROM nodes n WHERE m.id=n.id);
