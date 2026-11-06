"""Relit un IFC produit par le module ouvrage et vérifie sa conformité.

Usage : python3 tools/check_ifc.py fichier.ifc [places_attendues]

Contrôles : validation du schéma IFC4 (IfcOpenShell), identifiants uniques et décompressables,
structure spatiale, types, propriétés Pset_SpaceParking et PKM_Stationnement, géoréférencement.
"""
import json
import logging
import sys

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.util.element as ue
import ifcopenshell.validate


def main(path, attendu=None):
    # 1. Validation du schéma, à partir du fichier (lecture stricte : nombre d'attributs, énumérations,
    # types, cardinalités et règles EXPRESS).
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(path, log, express_rules=True)
    erreurs = [s for s in log.statements if str(s.get("level", "")).lower() == "error"]
    assert not erreurs, json.dumps(erreurs[:5], default=str, indent=1)
    f = ifcopenshell.open(path)
    assert f.schema == "IFC4", f.schema

    # 2. Identifiants : uniques, 22 caractères, réversibles.
    gids = [e.GlobalId for e in f.by_type("IfcRoot")]
    assert len(gids) == len(set(gids)), "GlobalId en double"
    for g in gids:
        assert len(g) == 22 and ifcopenshell.guid.compress(ifcopenshell.guid.expand(g)) == g, g

    # 3. Structure spatiale.
    (projet,) = f.by_type("IfcProject")
    (site,) = f.by_type("IfcSite")
    (bat,) = f.by_type("IfcBuilding")
    niveaux = f.by_type("IfcBuildingStorey")
    assert niveaux and all(ue.get_aggregate(n) == bat for n in niveaux)
    espaces = f.by_type("IfcSpace")
    places = [e for e in espaces if not ue.get_psets(e)["Pset_SpaceParking"]["IsAisle"]]
    allees = [e for e in espaces if ue.get_psets(e)["Pset_SpaceParking"]["IsAisle"]]
    assert all(e.PredefinedType == "PARKING" for e in espaces)
    assert all(ue.get_aggregate(e) in niveaux for e in espaces)
    for e in f.by_type("IfcSlab") + f.by_type("IfcColumn"):
        assert ue.get_container(e) in niveaux, e
        assert ue.get_type(e) is not None, e
    if attendu is not None:
        assert len(places) == attendu, (len(places), attendu)

    # 4. Propriétés.
    for e in places:
        ps = ue.get_psets(e)
        assert ps["Pset_SpaceParking"]["ParkingUnits"] == 1
        assert ps["PKM_Stationnement"]["CategorieNF"] in (1, 2, 3)

    # 5. Géoréférencement.
    mc = f.by_type("IfcMapConversion")
    crs = mc[0].TargetCRS.Name if mc else "local"

    print(
        f"{path} : IFC4 valide · {len(places)} places · {len(allees)} allées · "
        f"{len(f.by_type('IfcColumn'))} poteaux · {len(f.by_type('IfcSlab'))} dalles · "
        f"{len(f.by_type('IfcGridAxis'))} axes · {len(f.by_type('IfcTypeObject'))} types · {crs}"
    )


if __name__ == "__main__":
    logging.disable(logging.WARNING)
    main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else None)
