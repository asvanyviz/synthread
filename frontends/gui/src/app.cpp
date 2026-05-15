#include "app.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QNetworkRequest>
#include <QUrl>
#include <QDebug>

// ── ApiClient ──

ApiClient::ApiClient(const QString &baseUrl, QObject *parent)
    : QObject(parent), m_nam(new QNetworkAccessManager(this)), m_baseUrl(baseUrl)
{
}

QNetworkReply* ApiClient::get(const QString &path) {
    QNetworkRequest req(QUrl(m_baseUrl + path));
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    return m_nam->get(req);
}

QNetworkReply* ApiClient::post(const QString &path, const QJsonObject &body) {
    QNetworkRequest req(QUrl(m_baseUrl + path));
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    QJsonDocument doc(body);
    return m_nam->post(req, doc.toJson());
}

void ApiClient::fetchStatus() {
    auto *reply = get("/status");
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        if (reply->error() != QNetworkReply::NoError) {
            emit errorOccurred(reply->errorString());
            return;
        }
        QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
        emit statusReceived(doc.object());
    });
}

void ApiClient::fetchPeers() {
    auto *reply = get("/api/peers");
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        if (reply->error() != QNetworkReply::NoError) {
            emit errorOccurred(reply->errorString());
            return;
        }
        QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
        emit peersReceived(doc.array());
    });
}

void ApiClient::connectPeer(const QString &peerAddr) {
    QJsonObject body;
    body["peer_id_or_addr"] = peerAddr;
    auto *reply = post("/api/peers/connect", body);
    connect(reply, &QNetworkReply::finished, reply, &QNetworkReply::deleteLater);
}

void ApiClient::sendMessage(const QString &to, const QString &text) {
    QJsonObject body;
    body["to"] = to;
    body["text"] = text;
    auto *reply = post("/api/chat/send", body);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        if (reply->error() == QNetworkReply::NoError) {
            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            emit messageSent(doc.object()["msg_id"].toString());
        } else {
            emit errorOccurred(reply->errorString());
        }
    });
}

void ApiClient::sendFriendRequest(const QString &peerId) {
    auto *reply = post("/api/peers/" + peerId + "/friend-request", QJsonObject());
    connect(reply, &QNetworkReply::finished, reply, &QNetworkReply::deleteLater);
}

void ApiClient::acceptFriend(const QString &peerId) {
    auto *reply = post("/api/peers/" + peerId + "/friend-accept", QJsonObject());
    connect(reply, &QNetworkReply::finished, reply, &QNetworkReply::deleteLater);
}

// ── SynthreadApp ──

SynthreadApp::SynthreadApp(QObject *parent)
    : QObject(parent),
      m_api(new ApiClient("http://127.0.0.1:7700", this)),
      m_refreshTimer(new QTimer(this))
{
    connect(m_api, &ApiClient::statusReceived, this, [this](const QJsonObject &status) {
        m_peerId = status["peer_id"].toString();
        m_uptime = QString::number(status["uptime_secs"].toInt()) + "s";
        m_connectedPeers = status["connected_peers"].toInt();
        m_knownPeers = status["known_peers"].toInt();
        m_friends = status["friends"].toInt();
        emit statusChanged();
    });

    connect(m_api, &ApiClient::peersReceived, this, [this](const QJsonArray &peers) {
        m_peers = peers;
        m_peerIds.clear();
        for (const auto &p : peers) {
            QJsonObject obj = p.toObject();
            QString label = obj["peer_id"].toString().left(16);
            if (obj["relationship"].toString() == "Friend") label += " 👤";
            if (obj["priority"].toBool()) label += " ⭐";
            m_peerIds.append(label);
        }
        emit peersChanged();
    });

    connect(m_api, &ApiClient::errorOccurred, this, [this](const QString &err) {
        qWarning() << "API error:" << err;
        if (m_peerId.isEmpty()) {
            m_peerId = "offline";
            emit statusChanged();
        }
    });

    connect(m_refreshTimer, &QTimer::timeout, this, &SynthreadApp::refresh);
    m_refreshTimer->start(5000);

    // Defer first refresh so QML loads first
    QTimer::singleShot(500, this, &SynthreadApp::refresh);
}

SynthreadApp::~SynthreadApp() = default;

void SynthreadApp::refresh() {
    m_api->fetchStatus();
    m_api->fetchPeers();
}

void SynthreadApp::connectToPeer(const QString &addr) {
    m_api->connectPeer(addr);
}

void SynthreadApp::sendMessage(const QString &to, const QString &text) {
    m_api->sendMessage(to, text);
}

void SynthreadApp::addFriend(const QString &peerId) {
    m_api->sendFriendRequest(peerId);
}

void SynthreadApp::acceptFriend(const QString &peerId) {
    m_api->acceptFriend(peerId);
}
