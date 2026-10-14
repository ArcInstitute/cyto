use std::path::{Path, PathBuf};

use cyto_cli::map::MultiPairedInput;
use cyto_map::{
    CrisprMapper, DetectionConfig, GexMapper, ProbeMapper, Unpositioned, WhitelistMapper,
    detect_crispr_geometry, detect_gex_geometry,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Exact-match whitelist: detection only needs enough sampled reads to match,
/// and the hamming-1 expansion of the 737k whitelist is very expensive to build.
///
/// reduces runtime during the tests with minimal impact on predictions
fn load_whitelist(root: &Path) -> WhitelistMapper<Unpositioned> {
    let path = root.join("data/metadata/737K-fixed-rna-profiling.txt.gz");
    WhitelistMapper::from_file(&path, true, 1, 1).unwrap()
}

#[test]
fn test_detect_gex_geometry_from_binseq() {
    let root = workspace_root();

    let gex_path = root.join("data/libraries/gex_probes.tsv");
    let probe_path = root.join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
    let input_path = root.join("data/sequencing/gex.cbq");

    let whitelist = load_whitelist(&root);
    let gex = GexMapper::from_file(&gex_path, 1).unwrap();
    let probe: ProbeMapper<Unpositioned> = ProbeMapper::from_file(&probe_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
        num_threads: 1,
    };

    let result = detect_gex_geometry(whitelist, gex, Some(probe), &input, &config).unwrap();

    // Expected V1 geometry: [barcode][umi:12] | [gex][:18][probe]
    assert_eq!(
        result.geometry_string, "[barcode][umi:12] | [gex][:18][probe]",
        "detected geometry should match GEX Flex V1"
    );

    assert!(result.remap_window >= 1);
    assert!(result.total_reads_sampled > 0);

    // Verify evidence has correct components
    let components: Vec<_> = result.evidence.iter().map(|e| e.component).collect();
    assert!(components.contains(&cyto_map::Component::Barcode));
    assert!(components.contains(&cyto_map::Component::Gex));
    assert!(components.contains(&cyto_map::Component::Probe));

    // Windowed match count/proportion must be >= the single-position values
    // (windowed sum over a range that contains best_pos can only grow).
    let gex_evidence = result
        .evidence
        .iter()
        .find(|e| e.component == cyto_map::Component::Gex)
        .unwrap();
    assert!(
        gex_evidence.windowed_match_count >= gex_evidence.match_count,
        "gex windowed_match_count ({}) must be >= match_count ({})",
        gex_evidence.windowed_match_count,
        gex_evidence.match_count,
    );
    assert!(
        gex_evidence.windowed_match_proportion >= gex_evidence.match_proportion,
        "gex windowed_match_proportion ({}) must be >= match_proportion ({})",
        gex_evidence.windowed_match_proportion,
        gex_evidence.match_proportion,
    );

    // Probe assertion is the high-value check: the V1 fixture has known
    // positional drift on [probe] (motivates this feature; see PR #246), so
    // the windowed probe count must STRICTLY exceed the single-position count.
    let probe_evidence = result
        .evidence
        .iter()
        .find(|e| e.component == cyto_map::Component::Probe)
        .unwrap();
    assert!(
        probe_evidence.windowed_match_count > probe_evidence.match_count,
        "V1 fixture should show probe positional drift: \
         windowed_match_count ({}) > match_count ({})",
        probe_evidence.windowed_match_count,
        probe_evidence.match_count,
    );
}

#[test]
fn test_detect_crispr_geometry_from_binseq() {
    let root = workspace_root();

    let crispr_path = root.join("data/libraries/crispr_guides.tsv");
    let input_path = root.join("data/sequencing/crispr.cbq");

    let whitelist = load_whitelist(&root);
    let crispr = CrisprMapper::from_file(&crispr_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
        num_threads: 1,
    };

    let result = detect_crispr_geometry(whitelist, crispr, None, &input, &config).unwrap();

    // Verify the geometry contains the expected CRISPR components
    let gs = &result.geometry_string;
    assert!(
        gs.contains("[barcode]"),
        "geometry should contain [barcode]: {gs}"
    );
    assert!(
        gs.contains("[umi:12]"),
        "geometry should contain [umi:12]: {gs}"
    );
    assert!(
        gs.contains("[anchor]"),
        "geometry should contain [anchor]: {gs}"
    );
    assert!(
        gs.contains("[protospacer]"),
        "geometry should contain [protospacer]: {gs}"
    );

    assert!(result.remap_window >= 1);
    assert!(result.total_reads_sampled > 0);

    // Verify evidence has correct components
    let components: Vec<_> = result.evidence.iter().map(|e| e.component).collect();
    assert!(components.contains(&cyto_map::Component::Barcode));
    assert!(components.contains(&cyto_map::Component::Anchor));
    assert!(components.contains(&cyto_map::Component::Protospacer));
}

#[test]
fn test_detect_gex_geometry_unprobed() {
    let root = workspace_root();

    let gex_path = root.join("data/libraries/gex_probes.tsv");
    let input_path = root.join("data/sequencing/gex.cbq");

    let whitelist = load_whitelist(&root);
    let gex = GexMapper::from_file(&gex_path, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
        num_threads: 1,
    };

    let result = detect_gex_geometry(whitelist, gex, None, &input, &config).unwrap();

    // Verify the geometry contains the expected GEX components without probe
    let gs = &result.geometry_string;
    assert!(
        gs.contains("[barcode]"),
        "geometry should contain [barcode]: {gs}"
    );
    assert!(
        gs.contains("[umi:12]"),
        "geometry should contain [umi:12]: {gs}"
    );
    assert!(gs.contains("[gex]"), "geometry should contain [gex]: {gs}");
    assert!(
        !gs.contains("[probe]"),
        "geometry should NOT contain [probe]: {gs}"
    );

    assert!(result.remap_window >= 1);
    assert!(result.total_reads_sampled > 0);

    // Verify evidence has correct components
    let components: Vec<_> = result.evidence.iter().map(|e| e.component).collect();
    assert!(components.contains(&cyto_map::Component::Barcode));
    assert!(components.contains(&cyto_map::Component::Gex));
}

#[test]
fn test_detect_crispr_geometry_probed() {
    let root = workspace_root();

    let crispr_path = root.join("data/libraries/crispr_guides.tsv");
    let probe_path = root.join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
    let input_path = root.join("data/sequencing/crispr.cbq");

    let whitelist = load_whitelist(&root);
    let crispr = CrisprMapper::from_file(&crispr_path, false, 1).unwrap();
    let probe: ProbeMapper<Unpositioned> = ProbeMapper::from_file(&probe_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
        num_threads: 1,
    };

    let result = detect_crispr_geometry(whitelist, crispr, Some(probe), &input, &config).unwrap();

    // Verify the geometry contains the expected CRISPR components with probe
    let gs = &result.geometry_string;
    assert!(
        gs.contains("[barcode]"),
        "geometry should contain [barcode]: {gs}"
    );
    assert!(
        gs.contains("[umi:12]"),
        "geometry should contain [umi:12]: {gs}"
    );
    assert!(
        gs.contains("[anchor]"),
        "geometry should contain [anchor]: {gs}"
    );
    assert!(
        gs.contains("[protospacer]"),
        "geometry should contain [protospacer]: {gs}"
    );
    assert!(
        gs.contains("[probe]"),
        "geometry should contain [probe]: {gs}"
    );

    assert!(result.remap_window >= 1);
    assert!(result.total_reads_sampled > 0);

    // Verify evidence has correct components
    let components: Vec<_> = result.evidence.iter().map(|e| e.component).collect();
    assert!(components.contains(&cyto_map::Component::Barcode));
    assert!(components.contains(&cyto_map::Component::Anchor));
    assert!(components.contains(&cyto_map::Component::Protospacer));
    assert!(components.contains(&cyto_map::Component::Probe));
}

#[test]
fn test_detect_gex_geometry_multi_lane_binseq() {
    // Two lanes from the same file: `sample_gex_reads` opens a fresh reader per
    // `inputs` entry, so a repeated path is two independent lanes. Detection must
    // yield the canonical geometry, one per-file result per lane, and a pooled
    // read count equal to the sum of the (identical) per-lane counts. Running
    // detection once and checking the per-lane invariant is both stronger and
    // ~2x faster than comparing against a separate single-lane baseline.
    let root = workspace_root();

    let gex_path = root.join("data/libraries/gex_probes.tsv");
    let probe_path = root.join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
    let path = root
        .join("data/sequencing/gex.cbq")
        .to_string_lossy()
        .to_string();

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
        num_threads: 1,
    };

    let whitelist = load_whitelist(&root);
    let gex = GexMapper::from_file(&gex_path, 1).unwrap();
    let probe: ProbeMapper<Unpositioned> = ProbeMapper::from_file(&probe_path, false, 1).unwrap();
    let input = MultiPairedInput {
        inputs: vec![path.clone(), path],
    };
    let result = detect_gex_geometry(whitelist, gex, Some(probe), &input, &config).unwrap();

    assert_eq!(
        result.geometry_string, "[barcode][umi:12] | [gex][:18][probe]",
        "two identical lanes must detect the canonical GEX Flex V1 geometry"
    );
    assert_eq!(result.per_file_results.len(), 2);
    // Identical lanes sample identical read counts...
    assert_eq!(
        result.per_file_results[0].total_reads_sampled,
        result.per_file_results[1].total_reads_sampled,
    );
    assert!(result.per_file_results[0].total_reads_sampled > 0);
    // ...and the pooled total is exactly their sum.
    let lane_sum: usize = result
        .per_file_results
        .iter()
        .map(|r| r.total_reads_sampled)
        .sum();
    assert_eq!(result.total_reads_sampled, lane_sum);
}

#[test]
fn test_detect_gex_geometry_multi_lane_fastx() {
    // FASTX input: each consecutive pair of files is one lane, so
    // [R1, R2, R1, R2] is two lanes -- exercises the `chunks(2)` sampling path.
    let root = workspace_root();

    let gex_path = root.join("data/libraries/gex_probes.tsv");
    let probe_path = root.join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
    let r1 = root
        .join("data/sequencing/gex_R1.fastq.gz")
        .to_string_lossy()
        .to_string();
    let r2 = root
        .join("data/sequencing/gex_R2.fastq.gz")
        .to_string_lossy()
        .to_string();

    let whitelist = load_whitelist(&root);
    let gex = GexMapper::from_file(&gex_path, 1).unwrap();
    let probe: ProbeMapper<Unpositioned> = ProbeMapper::from_file(&probe_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![r1.clone(), r2.clone(), r1, r2],
    };
    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
        num_threads: 1,
    };

    let result = detect_gex_geometry(whitelist, gex, Some(probe), &input, &config).unwrap();

    assert_eq!(
        result.geometry_string, "[barcode][umi:12] | [gex][:18][probe]",
        "FASTX detection should match GEX Flex V1"
    );
    assert_eq!(result.per_file_results.len(), 2);
}
