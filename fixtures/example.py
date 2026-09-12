from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.linear_model import LogisticRegression

# Fit preprocessing inside each training fold to avoid validation leakage.
model = make_pipeline(StandardScaler(), LogisticRegression(C=0.5, max_iter=1000))
