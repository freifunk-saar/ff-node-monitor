CREATE TABLE monitors
(
  node character varying NOT NULL,
  email character varying NOT NULL
);
ALTER TABLE monitors ADD PRIMARY KEY (node, email);

CREATE TABLE nodes
(
  node character varying NOT NULL PRIMARY KEY,
  name character varying NOT NULL,
  state boolean NOT NULL
);
