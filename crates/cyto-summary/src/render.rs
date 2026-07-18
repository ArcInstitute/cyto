//! Render `SampleReport`s into self-contained HTML.
//!
//! Design: a white "sequencing QC printout" -- flat paper, hairline rules
//! instead of filled cards, all numeric data set in tabular monospace, a single
//! restrained accent. Layout follows the flow single-cell users know from
//! Cell Ranger (headline metrics, then a metrics-left / barcode-rank-plot-right
//! block, then mapping and per-probe detail) without copying its styling.
//!
//! No external assets: CSS is inlined and plots are hand-built SVG, so a report
//! is a single portable file. Entry points: [`render_sample`] and
//! [`render_master`].

use std::fmt::Write as _;

use crate::model::{ProbeSummary, SampleKind, SampleReport};

/// Muted, print-leaning categorical palette for barcode-rank lines.
const PALETTE: [&str; 8] = [
    "#0b6a83", "#b4531f", "#3b6b35", "#7a4fa3", "#9c2f4a", "#2f5f8a", "#8a6d1f", "#496b6b",
];

const CSS: &str = r#"
:root{
  --paper:#ffffff;--ink:#14181c;--muted:#727a82;--faint:#9aa1a8;
  --rule:#e7e9ec;--rule-strong:#d2d6da;--accent:#0b6a83;
  --good:#2f6b3a;--warn:#9a6a15;--bad:#a23b3b;--track:#f1f2f4;--hover:#fafbfc;
}
*{box-sizing:border-box}
html{-webkit-text-size-adjust:100%}
body{margin:0;background:var(--paper);color:var(--ink);
  font:14px/1.55 "Helvetica Neue",Helvetica,Arial,sans-serif;-webkit-font-smoothing:antialiased}
.page{max-width:1060px;margin:0 auto;padding:44px 32px 72px}
.mono{font-family:ui-monospace,"SF Mono","JetBrains Mono",Menlo,Consolas,monospace;font-variant-numeric:tabular-nums}
a{color:var(--accent);text-decoration:none;border-bottom:1px solid rgba(11,106,131,.32)}
a:hover{border-bottom-color:var(--accent)}

.masthead{display:flex;justify-content:space-between;align-items:flex-end;gap:20px;
  padding-bottom:14px;border-bottom:2px solid var(--ink)}
.title{font-size:30px;font-weight:700;letter-spacing:-.02em;line-height:1.04;margin:0}
.kind{margin-left:12px;font-size:11px;font-weight:700;letter-spacing:.09em;text-transform:uppercase;color:var(--accent)}
.sub{color:var(--muted);font-size:12.5px;margin-top:7px;font-family:ui-monospace,"SF Mono",Menlo,Consolas,monospace}
.brand{text-align:right;color:var(--faint);font-size:11.5px;white-space:nowrap;line-height:1.5}
.brand b{display:block;color:var(--ink);font-weight:700;letter-spacing:.14em;text-transform:uppercase;font-size:12px}

.metrics{display:flex;flex-wrap:wrap;margin:30px 0 4px;
  border-top:1px solid var(--rule);border-bottom:1px solid var(--rule)}
.metric{flex:1 1 0;min-width:132px;padding:16px 20px 15px;border-left:1px solid var(--rule)}
.metric:first-child{border-left:none;padding-left:0}
.metric .v{font-size:23px;font-weight:600;letter-spacing:-.01em;
  font-family:ui-monospace,"SF Mono",Menlo,Consolas,monospace;font-variant-numeric:tabular-nums}
.metric .k{margin-top:4px;font-size:10.5px;letter-spacing:.07em;text-transform:uppercase;color:var(--muted)}

.section{margin-top:40px}
.eyebrow{display:flex;align-items:center;gap:14px;margin-bottom:16px}
.eyebrow h2{font-size:12px;font-weight:700;letter-spacing:.11em;text-transform:uppercase;margin:0}
.eyebrow .ln{flex:1;height:1px;background:var(--rule-strong)}
.hint{color:var(--muted);font-size:12px;margin:-6px 0 16px;max-width:70ch}

.split{display:grid;grid-template-columns:minmax(0,320px) minmax(0,1fr);gap:36px;align-items:start}
@media(max-width:780px){.split{grid-template-columns:1fr;gap:24px}}

.keys{width:100%;border-collapse:collapse}
.keys td{padding:8px 0;border-bottom:1px solid var(--rule);font-size:13px;color:var(--muted)}
.keys td:last-child{text-align:right;color:var(--ink);
  font-family:ui-monospace,"SF Mono",Menlo,Consolas,monospace;font-variant-numeric:tabular-nums}
.keys tr:last-child td{border-bottom:none}

table.data{width:100%;border-collapse:collapse}
table.data th,table.data td{padding:9px 14px;white-space:nowrap;font-size:13px}
table.data th{font-size:10.5px;letter-spacing:.06em;text-transform:uppercase;color:var(--muted);
  font-weight:700;text-align:right;border-bottom:1px solid var(--rule-strong)}
table.data td{text-align:right;border-bottom:1px solid var(--rule)}
table.data th:first-child,table.data td:first-child{text-align:left}
table.data td:not(:first-child){font-family:ui-monospace,"SF Mono",Menlo,Consolas,monospace;font-variant-numeric:tabular-nums}
table.data tbody tr:last-child td{border-bottom:none}
table.data tbody tr:hover td{background:var(--hover)}
.overflow{overflow-x:auto}

.bars{margin-top:2px}
.bar{display:grid;grid-template-columns:200px 1fr 168px;align-items:center;gap:16px;padding:5px 0}
.bar .lbl{font-size:13px}
.bar .track{height:7px;background:var(--track)}
.bar .fill{height:100%}
.bar .val{font-family:ui-monospace,"SF Mono",Menlo,Consolas,monospace;font-variant-numeric:tabular-nums;
  font-size:12px;color:var(--muted);text-align:right}
.subhead{font-size:11px;letter-spacing:.05em;text-transform:uppercase;color:var(--faint);margin:16px 0 8px}
@media(max-width:780px){.bar{grid-template-columns:130px 1fr 120px;gap:10px}}
.bars2{display:grid;grid-template-columns:1fr 1fr;gap:0 48px}
.bars2 .bar{grid-template-columns:150px 1fr 122px;gap:12px}
@media(max-width:780px){.bars2{grid-template-columns:1fr}}
.cols2{display:grid;grid-template-columns:1fr 1fr;gap:20px 44px;align-items:start}
.cols2 .section{margin-top:0}
@media(max-width:780px){.cols2{grid-template-columns:1fr}}

figure{margin:0}
figcaption{color:var(--muted);font-size:11.5px;margin-top:10px}
svg.plot{display:block;width:100%;height:auto}

.alert{border:1px solid var(--rule);border-left:3px solid var(--warn);
  padding:11px 15px;margin:22px 0 0;font-size:13px;color:#4a4f55}
.alert b{color:var(--warn);font-weight:700}

.foot{margin-top:60px;padding-top:16px;border-top:1px solid var(--rule);
  color:var(--faint);font-size:11.5px;display:flex;justify-content:space-between;gap:16px}
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

/// Compact magnitude (e.g. `10.72B`) for headline figures.
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

/// "Genes"/"Guides" label with a suffix, e.g. `feature_word(kind, "detected")`.
fn feature_word(kind: SampleKind, suffix: &str) -> String {
    let noun = if kind == SampleKind::Crispr {
        "Guides"
    } else {
        "Genes"
    };
    format!("{noun} {suffix}")
}

fn metric(v: &str, k: &str) -> String {
    format!(
        "<div class=\"metric\"><div class=\"v\">{}</div><div class=\"k\">{}</div></div>",
        esc(v),
        esc(k)
    )
}

fn eyebrow(title: &str) -> String {
    format!(
        "<div class=\"eyebrow\"><h2>{}</h2><div class=\"ln\"></div></div>",
        esc(title)
    )
}

fn bar(label: &str, value: &str, frac: f64, color: &str) -> String {
    let w = (frac.clamp(0.0, 1.0) * 100.0).max(0.0);
    format!(
        "<div class=\"bar\"><div class=\"lbl\">{}</div>\
         <div class=\"track\"><div class=\"fill\" style=\"width:{w:.2}%;background:{color}\"></div></div>\
         <div class=\"val\">{}</div></div>",
        esc(label),
        esc(value)
    )
}

fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\
         \n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         \n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<div class=\"page\">\n{body}\n</div>\n</body>\n</html>\n",
        esc(title),
    )
}

fn footer(left: &str) -> String {
    format!(
        "<div class=\"foot\"><span>{}</span><span class=\"mono\">cyto summary v{}</span></div>",
        esc(left),
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

    let (w, h) = (720.0_f64, 300.0_f64);
    let (pl, pr, pt, pb) = (54.0_f64, 14.0_f64, 12.0_f64, 38.0_f64);
    let iw = w - pl - pr;
    let ih = h - pt - pb;
    let lx = (max_rank as f64).log10().max(1e-9);
    let ly = ((max_umis + 1) as f64).log10().max(1e-9);
    let px = |rank: u64| pl + (rank as f64).log10().clamp(0.0, lx) / lx * iw;
    let py = |umis: u64| pt + (1.0 - ((umis + 1) as f64).log10().clamp(0.0, ly) / ly) * ih;

    let mut s = String::new();
    let _ = write!(
        s,
        "<svg class=\"plot\" viewBox=\"0 0 {w} {h}\" preserveAspectRatio=\"xMidYMid meet\" role=\"img\" aria-label=\"Barcode rank plot\">"
    );
    // x gridlines (powers of 10)
    let mut e = 0u32;
    while 10u64.pow(e) <= max_rank {
        let x = px(10u64.pow(e));
        let _ = write!(
            s,
            "<line x1=\"{x:.1}\" y1=\"{pt}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"var(--rule)\"/>",
            pt + ih
        );
        let _ = write!(
            s,
            "<text x=\"{x:.1}\" y=\"{:.1}\" fill=\"var(--faint)\" font-size=\"10.5\" font-family=\"ui-monospace,Menlo,monospace\" text-anchor=\"middle\">{}</text>",
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
            "<line x1=\"{pl}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"var(--rule)\"/>",
            pl + iw
        );
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"var(--faint)\" font-size=\"10.5\" font-family=\"ui-monospace,Menlo,monospace\" text-anchor=\"end\">{}</text>",
            pl - 7.0,
            y + 3.5,
            compact(10u64.pow(e))
        );
        e += 1;
    }
    // axes (drawn over gridlines)
    let _ = write!(
        s,
        "<line x1=\"{pl}\" y1=\"{pt}\" x2=\"{pl}\" y2=\"{:.1}\" stroke=\"var(--rule-strong)\"/>",
        pt + ih
    );
    let _ = write!(
        s,
        "<line x1=\"{pl}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"var(--rule-strong)\"/>",
        pt + ih,
        pl + iw,
        pt + ih
    );
    // axis titles
    let _ = write!(
        s,
        "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"middle\">barcode rank</text>",
        pl + iw / 2.0,
        h - 3.0
    );
    let _ = write!(
        s,
        "<text transform=\"rotate(-90 11 {:.1})\" x=\"11\" y=\"{:.1}\" fill=\"var(--muted)\" font-size=\"11\" text-anchor=\"middle\">UMIs per barcode</text>",
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
            "<polyline points=\"{pts}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.6\" stroke-linejoin=\"round\"/>"
        );
    }
    // legend (top-right, over the empty high-rank corner)
    let mut ly0 = pt + 10.0;
    for (i, p) in with_data.iter().enumerate() {
        let color = PALETTE[i % PALETTE.len()];
        let x = pl + iw - 90.0;
        let _ = write!(
            s,
            "<line x1=\"{x:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{color}\" stroke-width=\"2\"/>",
            ly0 - 3.5,
            x + 16.0,
            ly0 - 3.5
        );
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{ly0:.1}\" fill=\"var(--ink)\" font-size=\"11\" font-family=\"ui-monospace,Menlo,monospace\">{}</text>",
            x + 22.0,
            esc(&p.name)
        );
        ly0 += 15.0;
    }
    s.push_str("</svg>");
    s
}

// ---------------------------------------------------------------------------
// per-sample sections
// ---------------------------------------------------------------------------

fn sample_metrics(r: &SampleReport) -> String {
    let mut m = String::new();
    let total_reads = r.mapping.as_ref().map_or(r.total_reads, |x| x.total_reads);
    m.push_str(&metric(&compact(total_reads), "Reads"));
    if let Some(mp) = &r.mapping {
        m.push_str(&metric(&pct(mp.mapped_reads_frac), "Mapped"));
    }
    if let Some(s) = r.overall_saturation {
        m.push_str(&metric(&pct(s), "Saturation"));
    }
    m.push_str(&metric(&compact(r.total_umis), "UMIs"));
    let barcodes: u64 = r.probes.iter().map(|p| p.n_barcodes).sum();
    m.push_str(&metric(&compact(barcodes), "Barcodes"));
    if let Some(det) = r.features_detected {
        m.push_str(&metric(&int(det), &feature_word(r.kind, "detected")));
    }
    if r.kind == SampleKind::Crispr {
        let assigned: u64 = r
            .probes
            .iter()
            .filter_map(|p| p.assignment.as_ref().map(|a| a.n_assigned))
            .sum();
        if assigned > 0 {
            m.push_str(&metric(&compact(assigned), "Cells assigned"));
        }
    }
    format!("<div class=\"metrics\">{m}</div>")
}

/// The Cell-Ranger-familiar block: library metrics on the left, barcode-rank on the right.
fn cells_section(r: &SampleReport) -> String {
    let svg = knee_svg(&r.probes);
    if svg.is_empty() {
        return String::new();
    }
    let barcodes: u64 = r.probes.iter().map(|p| p.n_barcodes).sum();
    let mut keys = String::new();
    let _ = write!(
        keys,
        "<tr><td>Barcodes observed</td><td>{}</td></tr>",
        int(barcodes)
    );
    let _ = write!(
        keys,
        "<tr><td>Total UMIs</td><td>{}</td></tr>",
        int(r.total_umis)
    );
    if barcodes > 0 {
        let _ = write!(
            keys,
            "<tr><td>Mean UMIs / barcode</td><td>{}</td></tr>",
            int(r.total_umis / barcodes)
        );
    }
    if r.total_umis > 0 {
        let _ = write!(
            keys,
            "<tr><td>Reads per UMI</td><td>{}</td></tr>",
            f1(r.total_reads as f64 / r.total_umis as f64)
        );
    }
    if let Some(s) = r.overall_saturation {
        let _ = write!(
            keys,
            "<tr><td>Sequencing saturation</td><td>{}</td></tr>",
            pct(s)
        );
    }
    if let Some(mp) = &r.mapping {
        let _ = write!(
            keys,
            "<tr><td>Reads mapped</td><td>{}</td></tr>",
            pct(mp.mapped_reads_frac)
        );
    }
    if let Some(det) = r.features_detected {
        let val = r
            .features_total
            .map_or_else(|| int(det), |tot| format!("{} / {}", int(det), int(tot)));
        let _ = write!(
            keys,
            "<tr><td>{}</td><td>{val}</td></tr>",
            feature_word(r.kind, "detected"),
        );
    }
    if r.kind == SampleKind::Crispr {
        let tested: u64 = r
            .probes
            .iter()
            .filter_map(|p| p.assignment.as_ref().map(|a| a.n_tested))
            .sum();
        if tested > 0 {
            let _ = write!(
                keys,
                "<tr><td>Cells tested (UMI threshold)</td><td>{}</td></tr>",
                int(tested)
            );
        }
    }
    let _ = write!(keys, "<tr><td>Probes</td><td>{}</td></tr>", r.probes.len());

    format!(
        "<section class=\"section\">{}\
         <div class=\"split\"><table class=\"keys\"><tbody>{keys}</tbody></table>\
         <figure>{svg}<figcaption>UMIs per barcode vs. rank (log&ndash;log), one line per probe. The knee separates cell-associated barcodes from background.</figcaption></figure></div></section>",
        eyebrow("Cells & barcodes"),
    )
}

fn mapping_section(r: &SampleReport) -> String {
    let Some(m) = &r.mapping else {
        return String::new();
    };
    let mut bars = bar(
        "Mapped",
        &format!("{} · {}", int(m.mapped_reads), pct(m.mapped_reads_frac)),
        m.mapped_reads_frac,
        "var(--accent)",
    );
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
    let mut reasons = String::new();
    for (label, count, frac) in breakdown {
        if count == 0 {
            continue;
        }
        reasons.push_str(&bar(
            label,
            &format!("{} · {}", compact(count), pct(frac)),
            frac,
            "var(--faint)",
        ));
    }
    if !reasons.is_empty() {
        let _ = write!(
            bars,
            "<div class=\"subhead\">Unmapped reads by reason</div><div class=\"bars2\">{reasons}</div>"
        );
    }
    format!(
        "<section class=\"section\">{}\
         <div class=\"hint\">{} reads total &middot; {} mapped &middot; {} unmapped. Reason shares are of unmapped reads and can overlap.</div>\
         <div class=\"bars\">{bars}</div></section>",
        eyebrow("Read mapping"),
        int(m.total_reads),
        int(m.mapped_reads),
        int(m.unmapped_reads),
    )
}

fn probe_section(r: &SampleReport) -> String {
    if r.probes.is_empty() {
        return String::new();
    }
    let is_crispr = r.kind == SampleKind::Crispr;
    let detected_hdr = if is_crispr {
        "Guides det."
    } else {
        "Genes det."
    };
    let has_detected = r.probes.iter().any(|p| p.features_detected.is_some());
    let mut head = String::from(
        "<tr><th>Probe</th><th>Barcodes</th><th>Reads</th><th>UMIs</th><th>Saturation</th>\
         <th>Median reads/bc</th><th>Median UMIs/bc</th><th>UMI corr.</th>",
    );
    if has_detected {
        let _ = write!(head, "<th>{detected_hdr}</th>");
    }
    if is_crispr {
        head.push_str("<th>Cells assigned</th><th>Dom. MOI</th>");
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
        if has_detected {
            let _ = write!(
                body,
                "<td>{}</td>",
                p.features_detected
                    .map_or_else(|| "&mdash;".to_string(), int),
            );
        }
        if is_crispr {
            if let Some(a) = &p.assignment {
                let _ = write!(
                    body,
                    "<td>{} · {}</td><td>{}</td>",
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
        "<section class=\"section\">{}\
         <div class=\"hint\">Saturation = 1 &minus; UMIs / reads (endpoint). Medians are per observed barcode.</div>\
         <div class=\"overflow\"><table class=\"data\"><thead>{head}</thead><tbody>{body}</tbody></table></div></section>",
        eyebrow("Per-probe metrics"),
    )
}

fn moi_section(r: &SampleReport) -> String {
    if r.kind != SampleKind::Crispr {
        return String::new();
    }
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
    let mut tail: u64 = 0;
    for (moi, count) in &agg {
        if *moi <= 8 {
            let frac_total = *count as f64 / total as f64;
            rows.push_str(&bar(
                &format!("{moi} guide(s)"),
                &format!("{} · {}", int(*count), pct(frac_total)),
                *count as f64 / max,
                "var(--accent)",
            ));
        } else {
            tail += *count;
        }
    }
    if tail > 0 {
        rows.push_str(&bar(
            "9+ guides",
            &format!("{} · {}", int(tail), pct(tail as f64 / total as f64)),
            tail as f64 / max,
            "var(--accent)",
        ));
    }
    format!(
        "<section class=\"section\">{}\
         <div class=\"hint\">Assigned cells by guides-per-cell, across all probes ({} cells).</div>\
         <div class=\"bars bars2\">{rows}</div></section>",
        eyebrow("Guide multiplicity"),
        int(total),
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

fn timing_section(r: &SampleReport) -> String {
    if r.timings.is_empty() {
        return String::new();
    }
    let mut agg: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for t in &r.timings {
        *agg.entry(t.module.clone()).or_insert(0.0) += t.elapsed;
    }
    let max = agg.values().copied().fold(0.0_f64, f64::max).max(1e-9);
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
            rows.push_str(&bar(name, &fmt_dur(*sec), *sec / max, "var(--faint)"));
            seen.push(name.to_string());
        }
    }
    for (name, sec) in &agg {
        if !seen.contains(name) {
            rows.push_str(&bar(name, &fmt_dur(*sec), *sec / max, "var(--faint)"));
        }
    }
    format!(
        "<section class=\"section\">{}\
         <div class=\"hint\">Wall-clock summed per module; post-mapping steps run in parallel across probes.</div>\
         <div class=\"bars bars2\">{rows}</div></section>",
        eyebrow("Pipeline timing"),
    )
}

fn library_section(r: &SampleReport) -> String {
    // The cell-barcode whitelist is not a feature library; hide it here.
    let libs: Vec<_> = r
        .libraries
        .iter()
        .filter(|l| l.name != "whitelist")
        .collect();
    if libs.is_empty() {
        return String::new();
    }
    let mut body = String::new();
    for l in libs {
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
        "<section class=\"section\">{}\
         <div class=\"overflow\"><table class=\"data\"><thead><tr><th>Library</th><th>Elements</th><th>Aggregated</th><th>Mate</th><th>Window</th><th>Match</th></tr></thead><tbody>{body}</tbody></table></div></section>",
        eyebrow("Reference libraries"),
    )
}

fn alerts(r: &SampleReport) -> String {
    if r.notes.is_empty() {
        return String::new();
    }
    let mut items = String::new();
    for n in &r.notes {
        let _ = write!(
            items,
            "<div class=\"alert\"><b>Note</b> &nbsp;{}</div>",
            esc(n)
        );
    }
    items
}

/// Render a full per-sample HTML report.
pub fn render_sample(r: &SampleReport) -> String {
    let meta = {
        let mut m = esc(&r.path);
        if let Some(s) = r.runtime_sec {
            let _ = write!(m, "  ·  mapping {}", fmt_dur(s));
        }
        m
    };
    let header = format!(
        "<div class=\"masthead\"><div><h1 class=\"title\">{}<span class=\"kind\">{}</span></h1>\
         <div class=\"sub\">{meta}</div></div>\
         <div class=\"brand\"><b>cyto</b>QC report</div></div>",
        esc(&r.sample),
        esc(r.kind.label()),
    );
    let body = format!(
        "{header}{}{}{}{}{}{}{}{}{}",
        alerts(r),
        sample_metrics(r),
        cells_section(r),
        mapping_section(r),
        probe_section(r),
        moi_section(r),
        timing_section(r),
        library_section(r),
        footer(&r.path),
    );
    page(&format!("{} — cyto QC report", r.sample), &body)
}

// ---------------------------------------------------------------------------
// master (cross-sample) report
// ---------------------------------------------------------------------------

/// Render one master column: a linked sample table for a single library type.
fn master_column(title: &str, subset: &[&(&SampleReport, String)], is_crispr: bool) -> String {
    let mut rows = String::new();
    for (r, href) in subset {
        let mapped = r
            .mapping
            .as_ref()
            .map_or_else(|| "&mdash;".to_string(), |m| pct(m.mapped_reads_frac));
        let sat = r
            .overall_saturation
            .map_or_else(|| "&mdash;".to_string(), pct);
        let reads = r.mapping.as_ref().map_or(r.total_reads, |m| m.total_reads);
        let _ = write!(
            rows,
            "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            esc(href),
            esc(&r.sample),
            compact(reads),
            mapped,
            compact(r.total_umis),
            sat,
        );
        if is_crispr {
            let assigned: u64 = r
                .probes
                .iter()
                .filter_map(|p| p.assignment.as_ref().map(|a| a.n_assigned))
                .sum();
            let cells = if assigned > 0 {
                compact(assigned)
            } else {
                "&mdash;".to_string()
            };
            let _ = write!(rows, "<td>{cells}</td>");
        }
        rows.push_str("</tr>");
    }
    let head = if is_crispr {
        "<tr><th>Sample</th><th>Reads</th><th>Mapped</th><th>UMIs</th><th>Sat.</th><th>Cells</th></tr>"
    } else {
        "<tr><th>Sample</th><th>Reads</th><th>Mapped</th><th>UMIs</th><th>Sat.</th></tr>"
    };
    format!(
        "<section class=\"section\">{}<div class=\"overflow\"><table class=\"data\"><thead>{head}</thead><tbody>{rows}</tbody></table></div></section>",
        eyebrow(title),
    )
}

/// Render an index over multiple samples, linking to each per-sample report.
pub fn render_master(run_name: &str, reports: &[(&SampleReport, String)]) -> String {
    let n = reports.len();
    let total_reads: u64 = reports
        .iter()
        .map(|(r, _)| r.mapping.as_ref().map_or(r.total_reads, |m| m.total_reads))
        .sum();
    let total_umis: u64 = reports.iter().map(|(r, _)| r.total_umis).sum();

    let mut m = String::new();
    m.push_str(&metric(&n.to_string(), "Samples"));
    m.push_str(&metric(&compact(total_reads), "Reads"));
    m.push_str(&metric(&compact(total_umis), "UMIs"));
    let metrics = format!("<div class=\"metrics\">{m}</div>");

    // Split samples into separate columns by library type.
    let kinds = [
        (SampleKind::Gex, "Gene expression", false),
        (SampleKind::Crispr, "CRISPR", true),
        (SampleKind::Unknown, "Other", false),
    ];
    let mut columns = String::new();
    let mut n_cols = 0;
    for (kind, title, is_crispr) in kinds {
        let subset: Vec<&(&SampleReport, String)> =
            reports.iter().filter(|(r, _)| r.kind == kind).collect();
        if subset.is_empty() {
            continue;
        }
        n_cols += 1;
        columns.push_str(&master_column(title, &subset, is_crispr));
    }
    let grid = if n_cols > 1 { "cols2" } else { "" };
    let body_grid = format!("<div class=\"{grid}\">{columns}</div>");

    let header = format!(
        "<div class=\"masthead\"><div><h1 class=\"title\">{}<span class=\"kind\">run</span></h1>\
         <div class=\"sub\">{n} sample{} · click a sample for its full report</div></div>\
         <div class=\"brand\"><b>cyto</b>QC report</div></div>",
        esc(run_name),
        if n == 1 { "" } else { "s" },
    );
    page(
        &format!("{run_name} — cyto run report"),
        &format!("{header}{metrics}{body_grid}{}", footer(run_name)),
    )
}
