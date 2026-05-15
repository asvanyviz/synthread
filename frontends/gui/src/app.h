#ifndef SYNTHREAD_GUI_APP_H
#define SYNTHREAD_GUI_APP_H

#include <QObject>
#include <QString>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonDocument>
#include <QTimer>
#include <QNetworkAccessManager>
#include <QNetworkReply>

class ApiClient : public QObject {
    Q_OBJECT
public:
    explicit ApiClient(const QString &baseUrl, QObject *parent = nullptr);

    void fetchStatus();
    void fetchPeers();
    void connectPeer(const QString &peerAddr);
    void sendMessage(const QString &to, const QString &text);
    void sendFriendRequest(const QString &peerId);
    void acceptFriend(const QString &peerId);

signals:
    void statusReceived(const QJsonObject &status);
    void peersReceived(const QJsonArray &peers);
    void errorOccurred(const QString &error);
    void messageSent(const QString &msgId);

private:
    QNetworkAccessManager *m_nam;
    QString m_baseUrl;

    QNetworkReply* get(const QString &path);
    QNetworkReply* post(const QString &path, const QJsonObject &body);
};

class SynthreadApp : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString peerId READ peerId NOTIFY statusChanged)
    Q_PROPERTY(QString uptime READ uptime NOTIFY statusChanged)
    Q_PROPERTY(int connectedPeers READ connectedPeers NOTIFY statusChanged)
    Q_PROPERTY(int knownPeers READ knownPeers NOTIFY statusChanged)
    Q_PROPERTY(int friends READ friends NOTIFY statusChanged)
    Q_PROPERTY(QStringList peerList READ peerList NOTIFY peersChanged)

public:
    explicit SynthreadApp(QObject *parent = nullptr);
    ~SynthreadApp();

    QString peerId() const { return m_peerId; }
    QString uptime() const { return m_uptime; }
    int connectedPeers() const { return m_connectedPeers; }
    int knownPeers() const { return m_knownPeers; }
    int friends() const { return m_friends; }
    QStringList peerList() const { return m_peerIds; }

    Q_INVOKABLE void refresh();
    Q_INVOKABLE void connectToPeer(const QString &addr);
    Q_INVOKABLE void sendMessage(const QString &to, const QString &text);
    Q_INVOKABLE void addFriend(const QString &peerId);
    Q_INVOKABLE void acceptFriend(const QString &peerId);

signals:
    void statusChanged();
    void peersChanged();
    void error(const QString &message);

private:
    ApiClient *m_api;
    QTimer *m_refreshTimer;

    QString m_peerId;
    QString m_uptime;
    int m_connectedPeers = 0;
    int m_knownPeers = 0;
    int m_friends = 0;
    QStringList m_peerIds;
    QJsonArray m_peers;
};

#endif // SYNTHREAD_GUI_APP_H
