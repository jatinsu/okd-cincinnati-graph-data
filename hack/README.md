# hack

This directory contains scripts that validate the data maintained in this repo.

* [util.py](util.py): It contains the common functions used by other Python Scripts.

* [validate-blocked-edges.py](validate-blocked-edges.py): It does basic blocked-edges validation and is executed in CI.
  It is ported unchanged from [openshift/cincinnati-graph-data](https://github.com/openshift/cincinnati-graph-data/tree/master/hack).

```bash
pip install -r hack/requirements.txt
./hack/validate-blocked-edges.py
```
