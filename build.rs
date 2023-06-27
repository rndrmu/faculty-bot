fn main() -> Result<(), Box<dyn std::error::Error>> {
    rosetta_build::config()
        .source("en", "i18n/en.json")
        .source("de", "i18n/de.json")
        .source("ja", "i18n/ja.json")
        .fallback("en")
        .generate()?;

    Ok(())
}
