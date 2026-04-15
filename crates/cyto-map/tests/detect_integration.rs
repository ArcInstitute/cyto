use std::path::PathBuf;

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

#[test]
fn test_detect_gex_geometry_from_binseq() {
    let root = workspace_root();

    let whitelist_path = root.join("data/metadata/737K-fixed-rna-profiling.txt.gz");
    let gex_path = root.join("data/libraries/gex_probes.tsv");
    let probe_path = root.join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
    let input_path = root.join("data/sequencing/gex.cbq");

    let whitelist = WhitelistMapper::from_file(&whitelist_path, false, 1, 1).unwrap();
    let gex = GexMapper::from_file(&gex_path, 1).unwrap();
    let probe: ProbeMapper<Unpositioned> = ProbeMapper::from_file(&probe_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
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
}

#[test]
fn test_detect_crispr_geometry_from_binseq() {
    let root = workspace_root();

    let whitelist_path = root.join("data/metadata/737K-fixed-rna-profiling.txt.gz");
    let crispr_path = root.join("data/libraries/crispr_guides.tsv");
    let input_path = root.join("data/sequencing/crispr.cbq");

    let whitelist = WhitelistMapper::from_file(&whitelist_path, false, 1, 1).unwrap();
    let crispr = CrisprMapper::from_file(&crispr_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
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

    let whitelist_path = root.join("data/metadata/737K-fixed-rna-profiling.txt.gz");
    let gex_path = root.join("data/libraries/gex_probes.tsv");
    let input_path = root.join("data/sequencing/gex.cbq");

    let whitelist = WhitelistMapper::from_file(&whitelist_path, false, 1, 1).unwrap();
    let gex = GexMapper::from_file(&gex_path, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
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

    let whitelist_path = root.join("data/metadata/737K-fixed-rna-profiling.txt.gz");
    let crispr_path = root.join("data/libraries/crispr_guides.tsv");
    let probe_path = root.join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
    let input_path = root.join("data/sequencing/crispr.cbq");

    let whitelist = WhitelistMapper::from_file(&whitelist_path, false, 1, 1).unwrap();
    let crispr = CrisprMapper::from_file(&crispr_path, false, 1).unwrap();
    let probe: ProbeMapper<Unpositioned> = ProbeMapper::from_file(&probe_path, false, 1).unwrap();

    let input = MultiPairedInput {
        inputs: vec![input_path.to_string_lossy().to_string()],
    };

    let config = DetectionConfig {
        num_reads: 10000,
        min_proportion: 0.10,
        remap_min_proportion: 0.01,
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
