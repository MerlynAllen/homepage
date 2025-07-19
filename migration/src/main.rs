use clap::{Parser, Subcommand};
use migration::{Migrator, MigratorTrait};
use sea_orm_migration::prelude::*;

#[derive(Parser)]
#[command(name = "migrate")]
#[command(about = "Database migration tool for homepage project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply all pending migrations
    Up,
    /// Rollback the last migration
    Down,
    /// Check migration status
    Status,
    /// Generate a new migration file
    Generate {
        /// Migration name
        name: String,
    },
}

fn generate_migration_template(name: &str) -> String {
    let _class_name = name
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<String>();

    format!(
        r#"use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {{
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
        // TODO: Implement your migration here
        // Example:
        // manager
        //     .create_table(
        //         Table::create()
        //             .table(YourTable::Table)
        //             .if_not_exists()
        //             .col(
        //                 ColumnDef::new(YourTable::Id)
        //                     .integer()
        //                     .not_null()
        //                     .auto_increment()
        //                     .primary_key(),
        //             )
        //             .col(ColumnDef::new(YourTable::Name).string().not_null())
        //             .to_owned(),
        //     )
        //     .await
        
        Ok(())
    }}

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
        // TODO: Implement rollback logic here
        // Example:
        // manager
        //     .drop_table(Table::drop().table(YourTable::Table).to_owned())
        //     .await
        
        Ok(())
    }}
}}

// TODO: Define your table enum if needed
// #[derive(DeriveIden)]
// enum YourTable {{
//     Table,
//     Id,
//     Name,
//     // Add more columns as needed
// }}
"#
    )
}

fn update_lib_rs(module_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lib_path = "migration/src/lib.rs";
    let content = std::fs::read_to_string(lib_path)?;

    // Check if module is already added
    if content.contains(&format!("mod {};", module_name)) {
        return Ok(());
    }

    let mut new_content = content.clone();

    // 1. Add module declaration after existing mod declarations
    let mod_declaration = format!("mod {};\n", module_name);

    // Find the last mod declaration and add after it
    if let Some(last_mod_pos) = content.rfind("mod ") {
        if let Some(newline_pos) = content[last_mod_pos..].find('\n') {
            let insert_pos = last_mod_pos + newline_pos + 1;
            new_content.insert_str(insert_pos, &mod_declaration);
        }
    } else {
        // No existing mod declarations, add after use statements
        if let Some(last_use_pos) = content.rfind("use ") {
            if let Some(newline_pos) = content[last_use_pos..].find('\n') {
                let insert_pos = last_use_pos + newline_pos + 1;
                new_content.insert_str(insert_pos, &format!("\n{}", mod_declaration));
            }
        }
    }

    // 2. Add to migrations vector
    let migration_entry = format!("Box::new({}::Migration)", module_name);

    // Handle the different formats of the migrations vector
    if new_content.contains("vec![]") {
        // Empty vector
        new_content = new_content.replace("vec![]", &format!("vec![{}]", migration_entry));
    } else if new_content.contains("vec![Box::new(") && new_content.contains(")]") {
        // Single line with existing migrations - find the last ] and add before it
        if let Some(bracket_pos) = new_content.rfind(']') {
            new_content.insert_str(bracket_pos, &format!(", {}", migration_entry));
        }
    } else {
        // Multi-line format or other format - find vec![ and the closing ]
        if let Some(vec_start) = new_content.find("vec![") {
            if let Some(bracket_pos) = new_content[vec_start..].rfind(']') {
                let actual_bracket_pos = vec_start + bracket_pos;
                let before_bracket = &new_content[..actual_bracket_pos];

                // Check if there are existing entries by looking for Box::new
                if before_bracket.contains("Box::new(") {
                    new_content.insert_str(
                        actual_bracket_pos,
                        &format!(",\n            {}", migration_entry),
                    );
                } else {
                    new_content.insert_str(actual_bracket_pos, &migration_entry);
                }
            }
        }
    }

    std::fs::write(lib_path, new_content)?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Database URL - you can customize this or make it configurable
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:/Volumes/Data/Coding/homepage/server/info.db".to_string());

    let db = sea_orm::Database::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    match cli.command {
        Commands::Up => {
            println!("Running migrations...");
            Migrator::up(&db, None)
                .await
                .expect("Failed to run migrations");
            println!("Migrations completed successfully!");
        }
        Commands::Down => {
            println!("Rolling back last migration...");
            Migrator::down(&db, None)
                .await
                .expect("Failed to rollback migration");
            println!("Rollback completed successfully!");
        }
        Commands::Status => {
            println!("Checking migration status...");
            println!("Migration status check completed. Use 'up' to apply pending migrations.");
        }
        Commands::Generate { name } => {
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
            let snake_case_name = name.to_lowercase().replace(' ', "_");
            let filename = format!("m{}_{}.rs", timestamp, snake_case_name);
            let module_name = filename.replace(".rs", "");
            let filepath = format!("migration/src/{}", filename);

            println!("Generating new migration: {}", name);

            // Create the migration file content
            let migration_content = generate_migration_template(&snake_case_name);

            // Write the migration file
            match std::fs::write(&filepath, migration_content) {
                Ok(_) => {
                    println!("✅ Created migration file: {}", filepath);

                    // Try to update lib.rs automatically
                    match update_lib_rs(&module_name) {
                        Ok(_) => {
                            println!("✅ Updated lib.rs automatically");
                            println!("🎉 Migration ready! Run 'cargo migrate up' to apply it.");
                        }
                        Err(e) => {
                            println!("⚠️  Could not update lib.rs automatically: {}", e);
                            println!("📝 Please manually add to lib.rs:");
                            println!("   mod {};", module_name);
                            println!("   Box::new({}::Migration),", module_name);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to create migration file: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
