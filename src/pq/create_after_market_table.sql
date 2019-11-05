CREATE TABLE IF NOT EXISTS after_market(
   symbol           VARCHAR(10)         NOT NULL,
   percentage       DOUBLE PRECISION    NOT NULL,
   date             TIMESTAMP WITH TIME ZONE,
   PRIMARY KEY      (symbol, date)
);

CREATE INDEX ON after_market (symbol);
CREATE INDEX ON after_market (percentage);
CREATE INDEX ON after_market (date);
