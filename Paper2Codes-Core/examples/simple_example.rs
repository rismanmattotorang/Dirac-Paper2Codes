// Example: Process a simple paper and generate code
//
// This example demonstrates the basic usage of Paper2Codes:
// 1. Parse a paper (text format)
// 2. Generate an implementation plan
// 3. Generate code for each module
// 4. Verify the generated code
//
// To run this example:
// 1. Set your API key: export OPENROUTER_API_KEY=sk-or-...
// 2. Run: cargo run --example simple_example

use paper2codes::{config::Config, coordinator::Coordinator, document::DocumentProcessor, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("paper2codes=info")
        .init();

    println!("Paper2Codes - Simple Example");
    println!("================================\n");

    // Load configuration
    println!("Loading configuration...");
    let config = Config::default();
    println!("✓ Configuration loaded\n");

    // Initialize document processor
    println!("Initializing document processor...");
    let processor = DocumentProcessor::new();
    println!("✓ Document processor ready\n");

    // Sample paper content
    let paper_content = r#"
Title: A Simple Sorting Algorithm

Abstract:
This paper presents a simple sorting algorithm for educational purposes.
The algorithm takes an array of integers and sorts them in ascending order
using a bubble sort approach.

1. Introduction
Sorting algorithms are fundamental to computer science. This paper
introduces a basic sorting algorithm suitable for teaching purposes.

2. Algorithm
Algorithm 1: Bubble Sort
Input: Array A of n integers
Output: Sorted array A

procedure BubbleSort(A):
    n = length(A)
    for i = 0 to n-1:
        for j = 0 to n-i-1:
            if A[j] > A[j+1]:
                swap A[j] and A[j+1]
    return A

3. Complexity Analysis
The time complexity of this algorithm is O(n²) in the worst case.
The space complexity is O(1) as it sorts in-place.

4. Conclusion
This paper presented a simple sorting algorithm with clear complexity bounds.
"#;

    // Parse the paper
    println!("Parsing paper...");
    let mut paper = processor
        .parse_text(
            paper_content,
            Some("A Simple Sorting Algorithm".to_string()),
        )
        .await?;
    println!("✓ Paper parsed: {}", paper.title);
    println!("  - {} segments", paper.segments.len());

    // Segment the paper
    println!("\nSegmenting paper...");
    processor.segment_paper(&mut paper)?;
    println!("✓ Paper segmented: {} sections", paper.segments.len());

    // Extract algorithms
    println!("\nExtracting algorithms...");
    paper.algorithms = processor.extract_algorithms(&paper);
    println!("✓ Found {} algorithms", paper.algorithms.len());
    for algo in &paper.algorithms {
        println!("  - {}", algo.name);
    }

    // Classify domain
    println!("\nClassifying domain...");
    let domain = processor.classify_domain(&paper).await?;
    println!("✓ Domain: {:?}", domain.domain);

    // Initialize coordinator
    println!("\nInitializing coordinator...");
    let mut coordinator = Coordinator::new(config).await?;
    println!("✓ Coordinator ready\n");

    // Process the paper (generate code)
    println!("Processing paper (this may take a few minutes)...");
    println!("This will:");
    println!("  1. Generate implementation plan");
    println!("  2. Analyze each module");
    println!("  3. Generate code for each module");
    println!("  4. Verify the generated code");
    println!();

    let repository = coordinator.process_paper(paper).await?;

    // Display results
    println!("\n✓ Processing complete!");
    println!("\nResults:");
    println!("--------");
    println!("Generated {} modules", repository.modules.len());
    println!("Output directory: {:?}", repository.root_path);
    println!("\nModules:");
    for module in &repository.modules {
        println!("  - {}", module.file_path.display());
        println!("    Language: {:?}", module.language);
        println!("    Lines: {}", module.content.lines().count());
    }

    println!("\n✓ Example complete!");
    println!("\nNext steps:");
    println!(
        "  1. Check the generated code in: {:?}",
        repository.root_path
    );
    println!("  2. Review and test the generated code");
    println!("  3. Iterate if needed");

    Ok(())
}
