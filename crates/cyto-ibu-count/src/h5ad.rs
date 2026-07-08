use std::path::Path;

use anndata::backend::{Compression, WriteConfig, set_default_write_config};
use anndata::data::DataFrameIndex;
use anndata::{AnnData, AnnDataOp};
use anndata_hdf5::H5;
use anyhow::Result;
use hashbrown::HashMap;
use ibu::Header;
use log::info;
use nalgebra_sparse::coo::CooMatrix;
use nalgebra_sparse::csr::CsrMatrix;

use crate::dedup::BarcodeIndexCounts;
use crate::extend_suffix;

/// Writes a barcode-by-feature count matrix directly to a `.h5ad` file.
///
/// Uses gzip rather than this crate's zstd default so the file is readable by
/// plain `h5py`/`anndata` without requiring the optional `hdf5plugin` package.
pub fn write_counts_h5ad<P: AsRef<Path>>(
    path: P,
    counts: &BarcodeIndexCounts,
    features: &[String],
    header: Header,
    suffix: Option<&str>,
) -> Result<()> {
    set_default_write_config(WriteConfig {
        compression: Some(Compression::Gzip(6)),
        block_size: None,
    });

    let n_var = features.len();
    let n_obs = counts.get_num_barcodes();
    let nnz = counts.get_nnz();

    let mut obs_names = Vec::with_capacity(n_obs);
    let mut bc_idx_map = HashMap::with_capacity(n_obs);
    let mut row_idx = Vec::with_capacity(nnz);
    let mut col_idx = Vec::with_capacity(nnz);
    let mut vals: Vec<u32> = Vec::with_capacity(nnz);
    let mut dbuf = Vec::default();

    for record in counts.iter_counts() {
        let obs_idx = if let Some(idx) = bc_idx_map.get(&record.barcode()) {
            *idx
        } else {
            dbuf.clear();
            bitnuc::from_2bit(record.barcode(), header.bc_len as usize, &mut dbuf)?;
            extend_suffix(&mut dbuf, suffix);

            let obs_idx = obs_names.len();
            obs_names.push(std::str::from_utf8(&dbuf)?.to_string());
            bc_idx_map.insert(record.barcode(), obs_idx);
            obs_idx
        };

        row_idx.push(obs_idx);
        col_idx.push(record.index() as usize);
        vals.push(record.count() as u32);
    }

    let coo = CooMatrix::try_from_triplets(n_obs, n_var, row_idx, col_idx, vals)
        .map_err(|e| anyhow::anyhow!("Unable to build sparse count matrix: {e}"))?;
    let csr = CsrMatrix::from(&coo);

    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.as_ref().exists() {
        std::fs::remove_file(path.as_ref())?;
    }

    let adata = AnnData::<H5>::new(path.as_ref())?;
    adata.set_x(csr)?;
    adata.set_obs_names(DataFrameIndex::from(obs_names))?;
    adata.set_var_names(DataFrameIndex::from(features.to_vec()))?;
    adata.close()?;

    info!(
        "Finished writing h5ad counts to {}",
        path.as_ref().display()
    );

    Ok(())
}
