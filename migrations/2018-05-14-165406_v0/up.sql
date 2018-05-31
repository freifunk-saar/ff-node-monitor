CREATE TABLE monitors
(
  id character varying NOT NULL,
  email character varying NOT NULL
);
ALTER TABLE monitors ADD PRIMARY KEY (id, email);

CREATE TABLE nodes
(
  id character varying NOT NULL PRIMARY KEY,
  name character varying NOT NULL,
  online boolean NOT NULL
);
