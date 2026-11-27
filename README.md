# Axirizon

Mod natif pour Horizon : outils de construction avec fantôme des blocs, inspirés
d'Axiom. Code original, sans lien avec Axiom ni son auteur.

**Jamais essayé en jeu.** GitHub Actions compile le mod, lance les tests de la
logique des outils et vérifie le paquet `.hmod`.

## Récupérer et installer

1. Onglet **Actions** > dernier « Build » vert > **Artifacts** > `axirizon-hmod`.
2. Décompresser : il contient `axirizon.hmod`.
3. L'ajouter dans la page Mods du launcher et autoriser les mods natifs.

Version visée : Minecraft 26.3, mise à jour du 20 novembre 11:18 UTC, commit `94bb847` (source :
`rdbtCVS/horizon`). Le launcher compile le mod lui-même et a besoin du kit de
cette version exacte.

## En jeu (solo, Overworld)

**B** active ou désactive. Chiffres **1 à 7** : outil. La liste des outils s'affiche en haut à gauche.

| Touche | Outil | Commandes |
|---|---|---|
| 1 | Move | clic gauche / droit : coins ; molette : déplacer ; clic droit : valider |
| 2 | Clone | comme Move, sans effacer la source |
| 3 | Stack | molette : nombre de copies |
| 4 | Smear | molette : étaler bloc par bloc |
| 5 | Extrude | clic droit : ajouter une couche ; gauche : retirer |
| 6 | Erase | deux coins, puis clic droit |
| 7 | Symmetry | clic droit : X, Z, les deux ; clic gauche : désactiver |

Ctrl+Z / Ctrl+Y : annuler, refaire. Ctrl+R : tourner. Ctrl+F : miroir.
Retour arrière : annuler la sélection. X, Y ou Z maintenu : bloquer un axe.

## Limites

- Les clics visent à la portée normale du joueur.
- Le fantôme est fait de cubes colorés (bleu : ajout, rouge : retrait), sans les
  textures ; seule une partie des très grandes formes est dessinée.
- Le contenu des coffres et les entités ne sont pas copiés ; déplacer un coffre
  fait tomber son contenu.
- La rotation et le miroir ne tournent pas l'orientation des blocs.
- Les chiffres changent aussi la case de la hotbar du jeu.
