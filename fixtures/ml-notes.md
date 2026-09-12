# Generalization and evaluation

A model overfits when it fits the training data better than unseen data. L2 regularization penalizes large weights. Cross-validation estimates performance across multiple training and validation splits. A final test set remains untouched until model selection has finished.

## Data leakage

Split data before fitting preprocessing transforms. Fit a scaler on the training partition and reuse it on validation and test partitions. Group related documents or conversations before splitting to avoid leakage between folds.

## Retrieval metrics

Hit@K indicates whether a relevant item is present among the top K results. Mean reciprocal rank averages the inverse rank of the first relevant result. Retrieval success does not imply that the generated answer is faithful to the retrieved evidence.
