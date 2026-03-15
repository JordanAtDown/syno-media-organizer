Connecte-toi au NAS Synology via SSH pour effectuer les opérations demandées.

## Configuration de connexion

Lis le fichier `.env.nas` à la racine du projet (s'il existe) pour obtenir les paramètres.
Format attendu :
```
NAS_HOST=Gringotts
NAS_USER=jcontent
NAS_PORT=22
NAS_KEY=~/.ssh/id_nas
```

La config SSH `~/.ssh/config` définit l'alias `nas` (HostName, User, IdentityFile).
Utilise donc simplement `ssh nas <commande>` pour toutes les opérations.

## Opérations disponibles

- **Statut + logs** : `ssh nas "sudo synopkg status syno-media-organizer"` + `ssh nas "tail -20 /var/packages/syno-media-organizer/var/syno-media-organizer.log"`
- **Démarrer** : `ssh nas "sudo synopkg start syno-media-organizer"`
- **Arrêter** : `ssh nas "sudo synopkg stop syno-media-organizer"`
- **Redémarrer** : `ssh nas "sudo synopkg restart syno-media-organizer"`
- **Installer un SPK** : `scp -P $NAS_PORT dist/syno-media-organizer-*.spk $NAS_USER@$NAS_HOST:/tmp/` puis `ssh nas "sudo synopkg install /tmp/syno-media-organizer-X.Y.Z.spk"`
- **Déployer la dernière version** : SCP depuis `dist/` + install + start
- **Lire config** : `ssh nas "cat /volume1/config/syno-media-organizer/config.toml"`
- **Lire logs complets** : `ssh nas "cat /var/packages/syno-media-organizer/var/syno-media-organizer.log"`

## Instructions

$ARGUMENTS

Si aucun argument fourni : affiche le statut du service et les 30 dernières lignes de log.
