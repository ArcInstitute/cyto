//! Render `SampleReport`s into self-contained HTML.
//!
//! No external assets: all CSS is inlined and plots are hand-built SVG, so a
//! report is a single portable file. Two entry points: [`render_sample`] for a
//! per-sample report and [`render_master`] for the cross-sample index.

use std::fmt::Write as _;

use crate::model::{ProbeSummary, SampleKind, SampleReport};

const PALETTE: [&str; 10] = [
    "#2563eb", "#dc2626", "#059669", "#d97706", "#7c3aed", "#0891b2", "#db2777", "#65a30d",
    "#9333ea", "#0d9488",
];

const CSS: &str = r#"
:root{--bg:#f6f7f9;--card:#fff;--ink:#1a1d21;--muted:#6b7280;--line:#e5e7eb;--accent:#2563eb;--good:#059669;--warn:#d97706;--bad:#dc2626;--track:#eef0f3}
@media (prefers-color-scheme:dark){:root{--bg:#0f1216;--card:#171b21;--ink:#e6e9ee;--muted:#9aa4b2;--line:#262c35;--accent:#60a5fa;--track:#20262e}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
.wrap{max-width:960px;margin:0 auto;padding:28px 20px 64px}
h1{font-size:22px;margin:0 0 2px}
h2{font-size:15px;margin:0 0 14px;font-weight:600}
a{color:var(--accent);text-decoration:none}
a:hover{text-decoration:underline}
.muted{color:var(--muted)}
.mono{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:12px}
.top{display:flex;align-items:baseline;gap:12px;flex-wrap:wrap;margin-bottom:20px}
.badge{display:inline-block;padding:2px 9px;border-radius:999px;font-size:12px;font-weight:600;background:var(--accent);color:#fff}
.badge.gex{background:#2563eb}.badge.crispr{background:#7c3aed}.badge.unknown{background:#6b7280}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:18px 20px;margin-bottom:16px}
.kpis{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin-bottom:16px}
.kpi{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:14px 16px}
.kpi .num{font-size:22px;font-weight:700;letter-spacing:-.02em}
.kpi .lbl{font-size:12px;color:var(--muted);margin-top:2px}
table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}
th,td{text-align:right;padding:7px 10px;border-bottom:1px solid var(--line);white-space:nowrap}
th:first-child,td:first-child{text-align:left}
th{font-size:12px;color:var(--muted);font-weight:600}
tbody tr:last-child td{border-bottom:none}
.overflow{overflow-x:auto}
.bar-row{display:flex;align-items:center;gap:10px;margin:6px 0}
.bar-label{width:170px;flex:none;font-size:13px}
.bar-track{flex:1;height:14px;background:var(--track);border-radius:7px;overflow:hidden}
.bar-fill{height:100%;border-radius:7px}
.bar-val{width:150px;flex:none;text-align:right;font-size:12px;font-variant-numeric:tabular-nums;color:var(--muted)}
.plot{width:100%;height:auto;overflow:visible}
.note{color:var(--warn);font-size:13px;margin:3px 0}
.foot{color:var(--muted);font-size:12px;margin-top:8px;text-align:center}
.hint{font-size:12px;color:var(--muted);margin:-6px 0 12px}
"#;

// ---------------------------------------------------------------------------
// formatting helpers
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Group digits with thousands separators.
fn int(n: u64) -> String {
    let s = n.to_string();
    let b = s.as_bytes();
    let len = b.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in b.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*c as char);
    }
    out
}

/// Compact magnitude (e.g. `10.72B`) for headline tiles.
fn compact(n: u64) -> String {
    let f = n as f64;
    if f >= 1e9 {
        format!("{:.2}B", f / 1e9)
    } else if f >= 1e6 {
        format!("{:.2}M", f / 1e6)
    } else if f >= 1e3 {
        format!("{:.1}K", f / 1e3)
    } else {
        n.to_string()
    }
}

fn pct(frac: f64) -> String {
    format!("{:.1}%", frac * 100.0)
}

fn f1(x: f64) -> String {
    format!("{x:.1}")
}

fn kpi(num: &str, lbl: &str) -> String {
    format!(
        "<div class=\"kpi\"><div class=\"num\">{}</div><div class=\"lbl\">{}</div></div>",
        esc(num),
        esc(lbl)
    )
}

fn hbar(label: &str, value: &str, frac: f64, color: &str) -> String {
    let w = (frac.clamp(0.0, 1.0) * 100.0).max(0.0);
    format!(
        "<div class=\"bar-row\"><div class=\"bar-label\">{}</div>\
         <div class=\"bar-track\"><div class=\"bar-fill\" style=\"width:{w:.2}%;background:{color}\"></div></div>\
         <div class=\"bar-val\">{}</div></div>",
        esc(label),
        esc(value)
    )
}

fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\
         \n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         \n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<div class=\"wrap\">\n{body}\n\
         <div class=\"foot\">Generated by <span class=\"mono\">cyto summary</span> v{}</div>\n\
         </div>\n</body>\n</html>\n",
        esc(title),
        env!("CARGO_PKG_VERSION"),
    )
}

// ---------------------------------------------------------------------------
// SVG barcode-rank knee plot
// ---------------------------------------------------------------------------

#[allow(clippy::many_single_char_names, clippy::too_many_lines)]
fn knee_svg(probes: &[ProbeSummary]) -> String {
    let with_data: Vec<&ProbeSummary> = probes.iter().filter(|p| !p.knee.is_empty()).collect();
    if with_data.is_empty() {
        return String::new();
    }
    let max_rank = with_data
        .iter()
        .flat_map(|p| p.knee.iter())
        .map(|k| k.rank)
        .max()
        .unwrap_or(1)
        .max(1);
    let max_umis = with_data
        .iter()
        .flat_map(|p| p.knee.iter())
        .map(|k| k.umis)
        .max()
        .unwrap_or(1)
        .max(1);

    let (w, h) = (780.0_f64, 340.0_f64);
    let (pl, pr, pt, pb) = (56.0_f64, 16.0_f64, 14.0_f64, 40.0_f64);
    let iw = w - pl - pr;
    let ih = h - pt - pb;
    let lx = (max_rank as f64).log10().max(1e-9);
    let ly = ((max_umis + 1) as f64).log10().max(1e-9);
    let px = |rank: u64| pl + (rank as f64).log10().clamp(0.0, lx) / lx * iw;
    let py = |umis: u64| pt + (1.0 - ((umis + 1) as f64).log10().clamp(0.0, ly) / ly) * ih;

    let mut s = String::new();
    let _ = write!(
        s,
        "<svg class=\"plot\" viewBox=\"0 0 {w} {h}\" preserveAspectRatio=\"xMidYMid meet\" role=\"img\">"
    );
    // axes
    let _ = write!(
        s,
        "<line x1=\"{pl}\" y1=\"{pt}\" x2=\"{pl}\" y2=\"{}\" stroke=\"var(--line)\"/>",
        pt + ih
    );
    let _ = write!(
        s,
        "<line x1=\"{pl}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"var(--line)\"/>",
        pt + ih,
        pl + iw,
        pt + ih
    );
    // x gridlines (powers of 10)
    let mut e = 0u32;
    while 10u64.pow(e) <= max_rank {
        let x = px(10u64.pow(e));
        let _ = write!(
            s,
            "<line x1=\"{x:.1}\" y1=\"{pt}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"var(--line)\" stroke-dasharray=\"2 3\" opacity=\"0.6\"/>",
            pt + ih
        );
        let _ = write!(
            s,
            "<text x=\"{x:.1}\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"middle\">{}</text>",
            pt + ih + 16.0,
            compact(10u64.pow(e))
        );
        e += 1;
    }
    // y gridlines (powers of 10)
    let mut e = 0u32;
    while 10u64.pow(e) <= max_umis {
        let y = py(10u64.pow(e));
        let _ = write!(
            s,
            "<line x1=\"{pl}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"var(--line)\" stroke-dasharray=\"2 3\" opacity=\"0.6\"/>",
            pl + iw
        );
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"end\">{}</text>",
            pl - 6.0,
            y + 3.5,
            compact(10u64.pow(e))
        );
        e += 1;
    }
    // axis titles
    let _ = write!(
        s,
        "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"middle\">Barcode rank</text>",
        pl + iw / 2.0,
        h - 4.0
    );
    let _ = write!(
        s,
        "<text transform=\"rotate(-90 12 {:.1})\" x=\"12\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"middle\">UMIs per barcode</text>",
        pt + ih / 2.0,
        pt + ih / 2.0
    );

    // polylines
    for (i, p) in with_data.iter().enumerate() {
        let color = PALETTE[i % PALETTE.len()];
        let pts: String = p
            .knee
            .iter()
            .map(|k| format!("{:.1},{:.1}", px(k.rank), py(k.umis)))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = write!(
            s,
            "<polyline points=\"{pts}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.8\"/>"
        );
    }
    // legend
    let mut ly0 = pt + 6.0;
    for (i, p) in with_data.iter().enumerate() {
        let color = PALETTE[i % PALETTE.len()];
        let x = pl + iw - 96.0;
        let _ = write!(
            s,
            "<rect x=\"{x:.1}\" y=\"{:.1}\" width=\"10\" height=\"10\" fill=\"{color}\" rx=\"2\"/>",
            ly0 - 8.0
        );
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{ly0:.1}\" fill=\"var(--ink)\" font-size=\"11\">{}</text>",
            x + 15.0,
            esc(&p.name)
        );
        ly0 += 15.0;
    }
    s.push_str("</svg>");
    s
}

// ---------------------------------------------------------------------------
// per-sample report
// ---------------------------------------------------------------------------

fn mapping_card(r: &SampleReport) -> String {
    let Some(m) = &r.mapping else {
        return String::new();
    };
    let mut bars = String::new();
    bars.push_str(&hbar(
        "Mapped",
        &format!("{} ({})", int(m.mapped_reads), pct(m.mapped_reads_frac)),
        m.mapped_reads_frac,
        "var(--good)",
    ));
    // unmapped breakdown (fractions are of total unmapped reads)
    let u = &m.unmapped;
    let breakdown = [
        ("Missing feature", u.missing_feature, u.missing_feature_frac),
        (
            "Failed UMI quality",
            u.failed_umi_qual,
            u.failed_umi_qual_frac,
        ),
        (
            "Missing whitelist (barcode)",
            u.missing_whitelist,
            u.missing_whitelist_frac,
        ),
        ("Missing probe", u.missing_probe, u.missing_probe_frac),
        ("UMI truncated", u.umi_truncated, u.umi_truncated_frac),
    ];
    let mut rows = String::new();
    for (label, count, frac) in breakdown {
        if count == 0 {
            continue;
        }
        rows.push_str(&hbar(
            label,
            &format!("{} ({} of unmapped)", int(count), pct(frac)),
            frac,
            "var(--bad)",
        ));
    }
    format!(
        "<div class=\"card\"><h2>Read mapping</h2>\
         <div class=\"hint\">{} total reads &middot; {} mapped &middot; {} unmapped</div>{bars}\
         <div style=\"height:6px\"></div><div class=\"muted\" style=\"font-size:12px;margin-bottom:6px\">Unmapped reads by reason (share of unmapped):</div>{rows}</div>",
        int(m.total_reads),
        int(m.mapped_reads),
        int(m.unmapped_reads),
    )
}

fn probe_table(r: &SampleReport) -> String {
    if r.probes.is_empty() {
        return String::new();
    }
    let is_crispr = r.kind == SampleKind::Crispr;
    let mut head = String::from(
        "<tr><th>Probe</th><th>Barcodes</th><th>Reads</th><th>UMIs</th><th>Saturation</th>\
         <th>Median reads/bc</th><th>Median UMIs/bc</th><th>UMI corr.</th>",
    );
    if is_crispr {
        head.push_str("<th>Cells assigned</th><th>Dominant MOI</th>");
    }
    head.push_str("</tr>");

    let mut body = String::new();
    for p in &r.probes {
        let umi_corr = p
            .umi_corrected_frac
            .map_or_else(|| "&mdash;".to_string(), pct);
        let _ = write!(
            body,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            esc(&p.name),
            int(p.n_barcodes),
            int(p.total_reads),
            int(p.total_umis),
            pct(p.saturation),
            f1(p.median_reads_per_barcode),
            f1(p.median_umis_per_barcode),
            umi_corr,
        );
        if is_crispr {
            if let Some(a) = &p.assignment {
                let _ = write!(
                    body,
                    "<td>{} ({})</td><td>{}</td>",
                    int(a.n_assigned),
                    pct(a.assigned_frac),
                    a.dominant_moi,
                );
            } else {
                body.push_str("<td>&mdash;</td><td>&mdash;</td>");
            }
        }
        body.push_str("</tr>");
    }
    format!(
        "<div class=\"card\"><h2>Per-probe metrics</h2>\
         <div class=\"hint\">Saturation = 1 &minus; UMIs / reads (endpoint). Medians are per observed barcode.</div>\
         <div class=\"overflow\"><table><thead>{head}</thead><tbody>{body}</tbody></table></div></div>"
    )
}

fn knee_card(r: &SampleReport) -> String {
    let svg = knee_svg(&r.probes);
    if svg.is_empty() {
        return String::new();
    }
    format!(
        "<div class=\"card\"><h2>Barcode-rank plot</h2>\
         <div class=\"hint\">UMIs per barcode vs. rank (log&ndash;log). The knee separates cell-associated barcodes from background.</div>{svg}</div>"
    )
}

fn moi_card(r: &SampleReport) -> String {
    if r.kind != SampleKind::Crispr {
        return String::new();
    }
    // Aggregate MOI distribution across probes.
    let mut agg: std::collections::BTreeMap<i64, u64> = std::collections::BTreeMap::new();
    for p in &r.probes {
        if let Some(a) = &p.assignment {
            for (moi, count) in &a.moi_distribution {
                *agg.entry(*moi).or_insert(0) += *count;
            }
        }
    }
    if agg.is_empty() {
        return String::new();
    }
    let max = agg.values().copied().max().unwrap_or(1).max(1) as f64;
    let total: u64 = agg.values().copied().sum();
    let mut rows = String::new();
    for (moi, count) in agg.iter().take(15) {
        let frac_total = *count as f64 / total as f64;
        rows.push_str(&hbar(
            &format!("MOI = {moi}"),
            &format!("{} ({})", int(*count), pct(frac_total)),
            *count as f64 / max,
            "var(--accent)",
        ));
    }
    format!(
        "<div class=\"card\"><h2>Guide multiplicity (MOI)</h2>\
         <div class=\"hint\">Number of assigned cells by guides-per-cell, across all probes ({} cells).</div>{rows}</div>",
        int(total)
    )
}

fn timings_card(r: &SampleReport) -> String {
    if r.timings.is_empty() {
        return String::new();
    }
    let mut agg: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for t in &r.timings {
        *agg.entry(t.module.clone()).or_insert(0.0) += t.elapsed;
    }
    let max = agg.values().copied().fold(0.0_f64, f64::max).max(1e-9);
    // Present in a stable, pipeline-ish order then any extras.
    let order = [
        "Mapping",
        "InitialSort",
        "UmiCorrection",
        "ReadsDump",
        "Counting",
        "ConversionH5ad",
        "DropletFiltering",
        "GuideAssignment",
    ];
    let mut rows = String::new();
    let mut seen: Vec<String> = Vec::new();
    for name in order {
        if let Some(sec) = agg.get(name) {
            rows.push_str(&hbar(name, &fmt_dur(*sec), *sec / max, "var(--accent)"));
            seen.push(name.to_string());
        }
    }
    for (name, sec) in &agg {
        if !seen.contains(name) {
            rows.push_str(&hbar(name, &fmt_dur(*sec), *sec / max, "var(--accent)"));
        }
    }
    format!(
        "<div class=\"card\"><h2>Pipeline timing</h2>\
         <div class=\"hint\">Wall-clock summed per module (post-mapping steps run in parallel across probes).</div>{rows}</div>"
    )
}

fn fmt_dur(sec: f64) -> String {
    if sec >= 3600.0 {
        format!("{:.1} h", sec / 3600.0)
    } else if sec >= 60.0 {
        format!("{:.1} min", sec / 60.0)
    } else {
        format!("{sec:.1} s")
    }
}

fn libraries_card(r: &SampleReport) -> String {
    if r.libraries.is_empty() {
        return String::new();
    }
    let mut body = String::new();
    for l in &r.libraries {
        let _ = write!(
            body,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(&l.name),
            int(l.total_elem),
            int(l.total_aggr),
            esc(&l.mate),
            l.window,
            if l.exact { "exact" } else { "fuzzy" },
        );
    }
    format!(
        "<div class=\"card\"><h2>Reference libraries</h2>\
         <div class=\"overflow\"><table><thead><tr><th>Library</th><th>Elements</th><th>Aggregated</th><th>Mate</th><th>Window</th><th>Match</th></tr></thead><tbody>{body}</tbody></table></div></div>"
    )
}

fn notes_card(r: &SampleReport) -> String {
    if r.notes.is_empty() {
        return String::new();
    }
    let mut items = String::new();
    for n in &r.notes {
        let _ = write!(items, "<div class=\"note\">&#9888; {}</div>", esc(n));
    }
    format!("<div class=\"card\"><h2>Notes</h2>{items}</div>")
}

fn sample_kpis(r: &SampleReport) -> String {
    let mut tiles = String::new();
    let total_reads = r.mapping.as_ref().map_or(r.total_reads, |m| m.total_reads);
    tiles.push_str(&kpi(&compact(total_reads), "Total reads"));
    if let Some(m) = &r.mapping {
        tiles.push_str(&kpi(&pct(m.mapped_reads_frac), "Reads mapped"));
    }
    if let Some(s) = r.overall_saturation {
        tiles.push_str(&kpi(&pct(s), "Sequencing saturation"));
    }
    tiles.push_str(&kpi(&r.probes.len().to_string(), "Probes"));
    tiles.push_str(&kpi(&compact(r.total_umis), "Total UMIs"));
    let barcodes: u64 = r.probes.iter().map(|p| p.n_barcodes).sum();
    tiles.push_str(&kpi(&compact(barcodes), "Barcodes observed"));
    if r.kind == SampleKind::Crispr {
        let assigned: u64 = r
            .probes
            .iter()
            .filter_map(|p| p.assignment.as_ref().map(|a| a.n_assigned))
            .sum();
        if assigned > 0 {
            tiles.push_str(&kpi(&compact(assigned), "Cells assigned"));
        }
    }
    format!("<div class=\"kpis\">{tiles}</div>")
}

/// Render a full per-sample HTML report.
pub fn render_sample(r: &SampleReport) -> String {
    let kind_class = match r.kind {
        SampleKind::Gex => "gex",
        SampleKind::Crispr => "crispr",
        SampleKind::Unknown => "unknown",
    };
    let runtime = r
        .runtime_sec
        .map(|s| format!(" &middot; mapping {}", fmt_dur(s)))
        .unwrap_or_default();
    let header = format!(
        "<div class=\"top\"><h1>{}</h1><span class=\"badge {kind_class}\">{}</span></div>\
         <div class=\"muted mono\" style=\"margin-top:-14px;margin-bottom:18px\">{}{runtime}</div>",
        esc(&r.sample),
        esc(r.kind.label()),
        esc(&r.path),
    );
    let body = format!(
        "{header}{}{}{}{}{}{}{}{}",
        sample_kpis(r),
        mapping_card(r),
        probe_table(r),
        knee_card(r),
        moi_card(r),
        timings_card(r),
        libraries_card(r),
        notes_card(r),
    );
    page(&format!("{} — cyto report", r.sample), &body)
}

// ---------------------------------------------------------------------------
// master (cross-sample) report
// ---------------------------------------------------------------------------

/// Render an index over multiple samples, linking to each per-sample report.
pub fn render_master(run_name: &str, reports: &[(&SampleReport, String)]) -> String {
    let n = reports.len();
    let total_reads: u64 = reports
        .iter()
        .map(|(r, _)| r.mapping.as_ref().map_or(r.total_reads, |m| m.total_reads))
        .sum();
    let total_umis: u64 = reports.iter().map(|(r, _)| r.total_umis).sum();

    let mut tiles = String::new();
    tiles.push_str(&kpi(&n.to_string(), "Samples"));
    tiles.push_str(&kpi(&compact(total_reads), "Total reads"));
    tiles.push_str(&kpi(&compact(total_umis), "Total UMIs"));
    let kpis = format!("<div class=\"kpis\">{tiles}</div>");

    let mut body_rows = String::new();
    for (r, href) in reports {
        let mapped = r
            .mapping
            .as_ref()
            .map_or_else(|| "&mdash;".to_string(), |m| pct(m.mapped_reads_frac));
        let sat = r
            .overall_saturation
            .map_or_else(|| "&mdash;".to_string(), pct);
        let reads = r.mapping.as_ref().map_or(r.total_reads, |m| m.total_reads);
        let assigned: u64 = r
            .probes
            .iter()
            .filter_map(|p| p.assignment.as_ref().map(|a| a.n_assigned))
            .sum();
        let cells = if r.kind == SampleKind::Crispr && assigned > 0 {
            int(assigned)
        } else {
            "&mdash;".to_string()
        };
        let _ = write!(
            body_rows,
            "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(href),
            esc(&r.sample),
            esc(r.kind.label()),
            int(reads),
            mapped,
            r.probes.len(),
            int(r.total_umis),
            sat,
            cells,
        );
    }
    let table = format!(
        "<div class=\"card\"><h2>Samples</h2><div class=\"overflow\"><table><thead>\
         <tr><th>Sample</th><th>Type</th><th>Reads</th><th>Mapped</th><th>Probes</th><th>UMIs</th><th>Saturation</th><th>Cells assigned</th></tr>\
         </thead><tbody>{body_rows}</tbody></table></div></div>"
    );

    let header = format!(
        "<div class=\"top\"><h1>{}</h1><span class=\"badge\">run</span></div>\
         <div class=\"muted\" style=\"margin-top:-14px;margin-bottom:18px\">{n} sample report{}</div>",
        esc(run_name),
        if n == 1 { "" } else { "s" },
    );
    page(
        &format!("{run_name} — cyto run report"),
        &format!("{header}{kpis}{table}"),
    )
}
