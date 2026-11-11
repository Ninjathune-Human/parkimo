# Parkimo : audit et direction

11 novembre 2026. Audit du dépôt `Ninjathune-Human/parkimo` (branches `main`, `giration`, `ouvrage-lot1`), des trois modules et des règles appliquées, puis direction retenue pour en faire une application de type TestFit : conception en 2D, visualisation 3D, pilotage par une IA.

## 1. Ce qui est fait ce jour

- Les trois branches sont réunies sur la branche `unification` (fusion de `ouvrage-lot1` dans `giration`, elle-même en avance sur `main`). 9 conflits résolus dans l'interface, 3 dans `src/lib.rs`, gabarit DXF régénéré (40 calques). Aucune fonction retirée.
- Suite complète verte sur cette branche : 167 tests (57 unitaires, 76 giration, 9 ouvrage, 25 qualité). Syntaxe de l'interface contrôlée, aucun identifiant orphelin après fusion.
- Un test corrigé : il mêlait un contrôle fonctionnel et un seuil de durée (60 ms par place), et échouait sur une machine à deux cœurs (89,7 ms) sans défaut du moteur. La durée n'est plus vérifiée que sur demande (`PARKIMO_PERF=1`).
- `main` et l'application en ligne ne sont pas touchés.

Non vérifié ici : le rendu dans un navigateur. La cible WebAssembly ne peut pas être installée dans cet environnement ; la page est compilée par l'intégration continue de la branche.

## 2. État du dépôt

| Constat | Détail | Conséquence |
| --- | --- | --- |
| Trois branches divergentes | `main` : surface seule (publiée). `giration` : +12 500 lignes. `ouvrage-lot1` : +9 300 lignes. Aucune ne contenait tout | Corrigé par `unification` |
| Pages compilées versionnées | `index.html`, `dist/index.html`, `dist/trame.html` (3,3 à 4,3 Mo chacune), plus `essai/` et `giration/` : 20 Mo sur les 23 Mo suivis | Conflit garanti à chaque fusion, historique alourdi |
| Interface en un fichier | `web/app.html` : 4 000 lignes, 260 Ko, environ 250 fonctions sur un état global, dessin Canvas 2D | Difficile à faire évoluer ; aucune couche de commandes, donc rien qu'une IA puisse piloter |
| Aucune 3D | Zéro occurrence de three.js, alors que le module ouvrage la prévoyait | À construire entièrement |
| Nom | La caisse Rust et le README s'appellent encore « Trame » | À renommer en Parkimo |
| `solve_flow` | Une fonction d'environ 450 lignes dans `src/lib.rs` | À découper par étape du pipeline |
| Durée des tests | `tests/quality.rs` : 11 minutes sur deux cœurs (le contrôle des virages rejoue la recherche de giration à chaque plan) | Intégration continue lente ; à séparer en tests rapides et tests longs |
| Intégration continue | Un seul flux, hors `main`, qui recompile et enregistre la page sur la branche. Pas de déploiement, pas de tag `deploye` | À remplacer |
| Deux girations | `src/giration/` (cinématique complète) et `src/ouvrage/giration.rs` (gabarit en arcs pour les rampes) | À réunir : les rampes doivent passer par le vrai simulateur |

## 3. Audit des modules

### Parking de surface

Acquis solides : reculs par limite, limites courbes, optimisation de la trame (4 000 à 5 000 variantes en 12 à 40 ms), places en bordure, réseau de voies et sens de circulation, espaces plantés, noues, bassins, traverses, fond IGN et cadastre, export DXF géoréférencé, estimation.

Limites :
- Pas de référentiel : les dimensions sont des curseurs libres, sans contrôle quand on passe sous un minimum.
- Places inclinées dimensionnées sur Neufert (p. 272) et non sur le tableau 2 de la NF P 91-100. Avec des places de 2,50 m le résultat reste conforme par la note 2 du tableau (demi-travée conservée : à 45°, 5,30 + 3,00 = 8,30 m pour 4,80 + 3,50 exigés), mais rien ne le vérifie.
- Rangement en chevron absent : 0,55 m perdu par rangée à 60°, 0,80 m à 45°.
- Places longitudinales absentes.
- Places PMR posées par groupes de quatre près de l'entrée, sans surlongueur de 1,20 m ni vérification du cheminement jusqu'à l'entrée.
- Ombrage, perméabilité, ombrières photovoltaïques, deux-roues et vélos : absents.

### Parking en structure

Acquis solides : valeurs normatives sourcées (`normes.rs`), trame et densité de poteaux, trois systèmes constructifs, onze implantations de rampes (droites, demi-niveaux, plateaux inclinés, tours hélicoïdales), escaliers et ascenseurs placés selon les distances d'évacuation, modèle BIM, exports IFC4 et DXF.

Limites :
- Plateau rectangulaire uniquement, places à 90° uniquement.
- Places PMR et IRVE comptées mais pas posées (un avertissement renvoie au concepteur).
- Un niveau type : pas de plan par niveau, pas de rampe d'accès depuis la voirie, pas de parc enterré.
- Beaucoup de règles déclarées et jamais appelées (voir § 4).
- Pas de 3D.

### Giration

Acquis solides : modèle cinématique (angle des roues dans l'état, braquage en roulant), A* hybride, test de toutes les places avec cinq verdicts, trait libre, édition de trajectoire, export DXF. Les décisions sont tracées dans `claude/giration-decisions.md`.

Limites :
- Un seul véhicule (Peugeot 3008), pas de sélecteur.
- Non branché sur l'ouvrage (rampes, tours).
- Coût : le contrôle des virages allonge chaque calcul complet et les tests.
- Les points à consolider du document de décisions restent ouverts (convention du diamètre de braquage, hauteur des bordures T3, comparaison Covadis).

## 4. Audit des règles

État : **appliqué** (le moteur s'en sert), **déclaré** (constante ou fonction écrite, jamais appelée), **absent**.

| Règle | Source | État |
| --- | --- | --- |
| Place 2,30 x 5,00, allée 5,00 à 90° | NF P 91-100, tab. 2 | Défauts 2,50 x 5,00 et 6,00 conformes ; aucun contrôle des réglages |
| Angles 75°, 60°, 45° et interpolation | NF P 91-100, tab. 2, § 3.1.2 | Déclaré (ouvrage) ; la surface suit Neufert |
| Chevron | NF P 91-100, fig. 3 à 5 | Absent |
| Longitudinal, voiles à gauche et à droite | NF P 91-100, tab. 2 notes 4 et 5 | Absent |
| Double sens au-delà de 20 places : +20 %, 5,00 m au moins | NF P 91-100, § 3.1.2 | Déclaré. Le moteur classe double sens dès 5,00 m (seuil NF P 91-120) : entre 5,00 et 5,99 m à 90°, il se trompe pour un parc public |
| Catégories 1, 2, 3 (80 / 10 / 10) | NF P 91-100, tab. 1 | Absent |
| Poteaux : retrait 0,65 m ou surlargeur | NF P 91-100, tab. 3 | Avertissement seulement ; surlargeur déclarée |
| Voiles : +0,20 et +0,30 m | NF P 91-100, fig. 1 | Déclaré |
| Hauteur libre 2,20 et 2,00 m | NF P 91-100, § 3.2 | Appliqué (hauteur d'étage minimale) |
| Voies hors stationnement, rayon extérieur 8,50 m | NF P 91-100, § 3.3 | Appliqué (rampes de l'ouvrage) |
| Rampes 18 %, raccords R 10 et R 15 | NF P 91-100, § 3.4 | Appliqué. Défaut réglé sur le maximum de 18 % ; Neufert conseille 15 % |
| Débouché sur voirie : 5 % sur 4,00 m | NF P 91-100, § 3.4, fig. 8 | Déclaré (pas de rampe d'accès) |
| Places adaptées : 2 % arrondi au supérieur, 3,30 x 5,00 | Arrêté du 20 avril 2017, art. 3 | Appliqué en surface ; compté seulement en ouvrage |
| Au-delà de 500 places : 10 au moins, arrêté municipal | Arrêté du 20 avril 2017, art. 3 | Déclaré |
| Surlongueur de 1,20 m en épi ou en bataille | Arrêté du 20 avril 2017, art. 3 | Déclaré |
| Dévers de 2 % au plus | Arrêté du 20 avril 2017, art. 3 | Avertissement (plateaux inclinés) |
| Distances aux escaliers 40 et 25 m, 50 et 30 m en parc largement ventilé | Règlement ERP, PS 13 | Appliqué. Valeurs du dépôt, non relues ce jour sur Légifrance |
| Compartiments de 3 000 m², désenfumage, IRVE par compartiment | Règlement ERP, PS 12, PS 18, guide PS 2018 | Déclaré |
| IRVE : 20 % pré-équipées, une borne par tranche de 20 places, une accessible PMR | Code de la construction, L. 113-12 et L. 113-13 | Appliqué en surface ; compté seulement en ouvrage. Cas résidentiel (100 %) absent |
| Parc extérieur de plus de 500 m² : moitié perméable, moitié ombragée | Code de l'urbanisme, L. 111-19-1 | Absent |
| Parc extérieur de plus de 1 500 m² : ombrières à énergie renouvelable sur la moitié | Loi du 10 mars 2023 (APER), art. 40 | Absent |
| Parc privatif | NF P 91-120 | Seul le seuil de 5,00 m est repris |

Lecture d'ensemble : le module ouvrage a le bon principe (chaque valeur porte sa source) mais n'en applique qu'une partie ; le module de surface applique des règles sans référentiel. Il faut un seul référentiel, commun, et un contrôle qui rende des constats.

## 5. Direction retenue

Ces décisions sont prises ; elles se corrigent à la relecture.

1. **Un seul tronc.** `unification` devient `main` une fois la page vérifiée dans un navigateur. Les pages compilées sortent du dépôt : publication par l'intégration continue, tag `deploye` déplacé sur le commit publié.
2. **Parkimo partout.** Caisse `parkimo`, modules `surface`, `ouvrage`, `giration`, `normes`, `commandes`, `scene3d`. Une seule caisse et un seul fichier WebAssembly tant que rien n'impose de la diviser.
3. **Un référentiel de règles commun.** Chaque valeur avec son unité et sa source, comme dans `normes.rs`. Un programme choisit le jeu applicable : parc public (NF P 91-100 et règles ERP), privatif (NF P 91-120), habitation. Le moteur rend des constats (règle, source, valeur mesurée, valeur exigée, éléments concernés) affichés dans un panneau Conformité, à la place des avertissements en texte libre.
4. **Tout passe par des commandes.** Chaque action (déplacer un sommet, changer l'angle, poser une traverse, passer en silo) devient une commande sérialisable appliquée au projet par le moteur Rust. L'interface, l'annulation, l'IA et le serveur MCP utilisent les mêmes commandes. C'est la condition pour qu'une IA agisse sur l'aménagement sans code particulier.
5. **IA par deux voies, mêmes outils.** Dans l'application : un champ de dialogue relié à l'API, dont les outils sont les commandes (lire le projet, lire le bilan, lister les constats, appliquer des commandes, comparer des variantes). Hors application : un serveur MCP en Rust, qui partage le moteur, travaille sur un fichier de projet et sait exporter DXF et IFC.
6. **La 3D est une vue, pas un second modèle.** Le moteur produit une description de scène (éléments typés, emprise, hauteurs, classe de matériau, clé BIM) ; three.js ne fait que la rendre. Trois rendus : esquisse (traits, arêtes, léger tremblé), maquette (carton blanc, ombres douces), réaliste (matériaux texturés, marquages, végétation, véhicules, soleil). La conception reste en 2D ; la sélection est partagée entre les deux vues.
7. **Interface.** Sources découpées en modules et assemblées en un seul fichier à la compilation (le déploiement reste un `index.html`). Une barre de commandes unique, un panneau contextuel, une palette de commandes qui sert aussi d'entrée au dialogue avec l'IA. Mêmes emplacements dans tous les modes, PC, iPad et iPhone, tactile et stylet.

## 6. Lots

| Lot | Contenu | Vérification |
| --- | --- | --- |
| 0. Unification | Fait | 167 tests verts |
| 1. Socle | Renommage, découpage en modules, référentiel de règles et panneau Conformité, couche de commandes, intégration continue et déploiement | Tests existants inchangés ; chaque constat du § 4 couvert par un test ; annuler et rétablir rejoués par commandes |
| 2. 3D | Description de scène en Rust, vue three.js, trois rendus, surface puis ouvrage (niveaux, rampes, noyaux, ossature) | Scène contrôlée par tests (nombre et hauteur des éléments) ; captures de référence dans un navigateur |
| 3. IA | Outils sur les commandes, dialogue dans l'application, serveur MCP | Scénarios rejoués : « passe les rangées à 60° », « ajoute un niveau », « rends le parc conforme » |
| 4. Aller plus loin | Surface : chevron, longitudinal, ombrières et perméabilité, deux-roues, cheminement PMR. Ouvrage : dalles non rectangulaires, PMR et IRVE posées, plan par niveau, compartiments, enterré. Giration : rampes et tours, sélecteur de véhicules, variantes comparées | Tests de capacité et de conformité par scénario |

## 7. Ce qui vous revient

- **Mise en ligne.** Remplacer `main` par `unification` change l'application publiée. Je le fais après contrôle dans un navigateur, sauf avis contraire.
- **Programme par défaut.** Parc public (NF P 91-100 et ERP) retenu. À changer si vos projets sont surtout privatifs.
- **IA dans l'application.** Elle demande une clé d'API saisie par l'utilisateur dans le navigateur ; le serveur MCP n'en demande pas.

## Sources réglementaires consultées ce jour

- NF P 91-100 (mai 1994), d'après le skill fourni.
- Arrêté du 20 avril 2017, art. 3 : fiches de la Fédération des aveugles de France et des DDT.
- Code de l'urbanisme, L. 111-19-1, et loi APER, art. 40 : ministère de la Transition écologique, photovoltaique.info, Seban et associés, CMS Francis Lefebvre.
- IRVE : fiches Legrand et ChargeGuru sur les articles L. 113-12 et L. 113-13 du code de la construction.
