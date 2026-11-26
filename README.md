# Axirizon

Outils de construction avec fantôme des blocs pour Horizon, inspirés d'Axiom.
Code original, sans lien avec Axiom ni son auteur.

**État : pas encore jouable.** La logique des outils est là ; le branchement
sur Horizon (clics, molette, écriture des blocs, dessin du fantôme) reste à écrire.

| Case | Outil | Commandes |
|---|---|---|
| 1 | Move | clic gauche / droit : coins ; molette : déplacer ; clic droit : valider |
| 2 | Clone | comme Move, sans effacer la source |
| 3 | Stack | molette : nombre de copies |
| 4 | Smear | molette : étaler bloc par bloc |
| 5 | Extrude | clic droit : ajouter une couche ; gauche : retirer |
| 6 | Erase | deux coins, puis clic droit |
| 7 | Symmetry | clic droit : X, Z, les deux ; clic gauche : désactiver |

Annuler / refaire, rotation et miroir sont gérés. La rotation et le miroir
déplacent les blocs mais ne tournent pas encore leur orientation (escaliers, bûches).
La sélection par faces au clic molette et la copie des entités ne sont pas faites.

La compilation et les tests passent uniquement par GitHub Actions.
