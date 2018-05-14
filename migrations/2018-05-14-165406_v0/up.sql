CREATE TABLE monitors
(
  node character varying NOT NULL,
  email character varying NOT NULL
);

CREATE TABLE nodes
(
  node character varying NOT NULL PRIMARY KEY,
  state bit(1) NOT NULL
);
