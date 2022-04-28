use std::path::Path;
use tantivy::schema::{Schema, TEXT, STORED, STRING, Field, INDEXED};
use tantivy::{Index, Document, IndexWriter, IndexReader};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{QueryParser};

const CIK: &str = "cik";
const NAMES: &str = "names";
const TICKERS: &str = "tickers";

fn default_schema() -> Schema {
    let mut builder = Schema::builder();
    builder.add_u64_field(CIK, INDEXED | STORED);
    builder.add_text_field(NAMES, TEXT | STORED);
    builder.add_text_field(TICKERS, STRING | STORED);
    builder.build().clone()
}

pub struct CikIndex {
    index: Index,
    schema: Schema,
    cik_field: Field,
    ticker_field: Field,
    name_field: Field,
    reader: IndexReader,
    parser: QueryParser
}

impl CikIndex {
    pub fn at_location(path: &Path) -> CikIndex {
        let schema = default_schema();
        let directory = MmapDirectory::open(path).unwrap();
        let index = Index::open_or_create(directory.clone(), schema.clone()).unwrap();

        let reader = index.reader().unwrap();
        let cik_field = schema.get_field(CIK).unwrap();
        let ticker_field = schema.get_field(TICKERS).unwrap();
        let name_field = schema.get_field(NAMES).unwrap();
        let parser = QueryParser::for_index(&index, vec![ticker_field, name_field]);
        CikIndex {
            index,
            schema,
            cik_field,
            ticker_field,
            name_field,
            reader,
            parser
        }
    }

    pub fn create_writer(&self) -> IndexWriter {
        self.index.writer(512_000_000).unwrap()
    }

    pub fn insert(&self, writer: &IndexWriter, cik: u64, tickers: &Vec<String>, names: &Vec<String>) {
        let mut document = Document::default();
        document.add_u64(self.cik_field, cik);
        for ticker in tickers {
            document.add_text(self.ticker_field, ticker);
        }
        for name in names {
            document.add_text(self.name_field, name);
        }
        // writer.delete_term(Term::from_field_u64(self.cik_field, cik));
        writer.add_document(document).unwrap();
    }

    pub fn commit(&self, writer: &mut IndexWriter) {
        writer.commit().unwrap();
    }

    pub fn search(&self, query: &str, limit: usize) -> CompanyQueryResult {
        let searcher = self.reader.searcher();
        let q = self.parser.parse_query(remove_special(query).as_str()).unwrap();
        let top_docs = searcher.search(&q, &TopDocs::with_limit(limit)).unwrap();
        let mut companies: Vec<Company> = Vec::with_capacity(top_docs.len());
        for (_score, doc_address) in top_docs {
            // Retrieve the actual content of documents given its `doc_address`.
            let retrieved_doc = searcher.doc(doc_address).unwrap();

            let cik: u64 = retrieved_doc.get_first(self.cik_field).map_or(0, |v| {v.as_u64().unwrap_or(0)});
            let mut names: Vec<String> = Vec::new();
            for item in retrieved_doc.get_all(self.name_field) {
                names.push(String::from(item.as_text().unwrap_or("err")));
            }
            let mut tickers: Vec<String> = Vec::new();
            for item in retrieved_doc.get_all(self.ticker_field) {
                tickers.push(String::from(item.as_text().unwrap_or("err")));
            }

            companies.push(Company {
                cik,
                names,
                tickers
            });


            log::info!("{} {}", _score, self.schema.to_json(&retrieved_doc));


        };
        CompanyQueryResult { companies }
    }
}

fn remove_special(query: &str) -> String {
    let mut q = String::from(query);
    q.retain(|c| c.is_digit(10) || c.is_alphabetic());
    q
}

pub struct CompanyQueryResult {
    companies: Vec<Company>
}

pub struct Company {
    cik: u64,
    names: Vec<String>,
    tickers: Vec<String>
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
