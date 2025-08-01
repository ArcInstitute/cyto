#!/usr/bin/env -S uv run --with anndata --script

import os
import sys


def get_paths(input_path: str) -> tuple[str, str, str]:
    if input_path.endswith(".mtx") or input_path.endswith(".mtx.gz"):
        mtx_path = input_path
        feature_path = mtx_path.replace(".mtx", ".features.tsv")
        barcode_path = mtx_path.replace(".mtx", ".barcodes.tsv")

        if not os.path.exists(feature_path):
            print(f"Error: Expected Feature file {feature_path} does not exist.")
            sys.exit(1)

        if not os.path.exists(barcode_path):
            print(f"Error: Expected Barcode file {barcode_path} does not exist.")
            sys.exit(1)
    else:
        if os.path.exists(os.path.join(input_path, "matrix.mtx")):
            mtx_path = os.path.join(input_path, "matrix.mtx")
            feature_path = os.path.join(input_path, "features.tsv")
            barcode_path = os.path.join(input_path, "barcodes.tsv")

        elif os.path.exists(os.path.join(input_path, "matrix.mtx.gz")):
            mtx_path = os.path.join(input_path, "matrix.mtx.gz")
            feature_path = os.path.join(input_path, "features.tsv.gz")
            barcode_path = os.path.join(input_path, "barcodes.tsv.gz")

        else:
            mtx_path_uncompressed = os.path.join(input_path, "matrix.mtx")
            mtx_path_compressed = os.path.join(input_path, "matrix.mtx.gz")
            sys.exit(
                f"Error: Expected Matrix file {mtx_path_uncompressed} or {mtx_path_compressed} does not exist."
            )

        if not os.path.exists(feature_path):
            print(f"Error: Expected Feature file {feature_path} does not exist.")
            sys.exit(1)

        if not os.path.exists(barcode_path):
            print(f"Error: Expected Barcode file {barcode_path} does not exist.")
            sys.exit(1)

    return (mtx_path, feature_path, barcode_path)


def main():
    import anndata as ad  # type: ignore
    import pandas as pd  # type: ignore

    if len(sys.argv) != 3:
        print("Usage: mtx_to_h5ad.py <input_directory / input_mtx> <output_path>")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    (mtx_path, feature_path, barcode_path) = get_paths(input_path)

    # note: mtx is gene x cell
    adata = ad.io.read_mtx(mtx_path)

    features = pd.read_csv(
        feature_path, sep="\t", header=None, index_col=0
    ).index.astype(str)
    features.name = "feature"
    adata.obs_names = features

    barcodes = pd.read_csv(
        barcode_path, sep="\t", header=None, index_col=0
    ).index.astype(str)
    barcodes.name = "barcode"
    adata.var_names = barcodes

    # write as cell x gene
    adata.T.write_h5ad(output_path)


if __name__ == "__main__":
    main()
