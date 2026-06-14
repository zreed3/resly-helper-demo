# API Inventory

This inventory is based on Resly's public documentation snapshots and embedded OpenAPI snippets.

## Base URLs

- Test: `https://test.api.resly.com.au`
- Production: `https://api.resly.com.au`

## Auth

`POST /token`

Body:

```json
{
  "accountId": "<account-id>",
  "key": "<api-key>"
}
```

The response includes a Bearer token, typically valid for 24 hours.

## MVP Endpoint Map

| CLI command | API path | Method | Notes |
| --- | --- | --- | --- |
| `account get` | `/property` | GET | Account/property information |
| `agents list` | `/agents` | GET | Agent/channel discovery |
| `room-types list` | `/room-types` | GET | Supports `showPhotos`, `portfolioId` |
| `room-types get` | `/room-types/{roomTypeId}` | GET | Exact room type read |
| `rooms list` | `/rooms` | GET | Physical room inventory |
| `rooms get` | `/rooms/{roomId}` | GET | Exact room read |
| `rate-plans list` | `/rate-plans` | GET | Rate plan discovery |
| `reservations list` | `/reservations` | GET | Requires `dateType`; date or timestamp bounds |
| `reservations get` | `/reservations/{reservationId}` | GET | Exact reservation read |
| `reservations in-house` | `/reservations-inhouse` | GET | In-house reservation list |
| `availability quote` | `/room-types`, `/rate-plans`, `/room-types/{roomTypeId}/inventory`, `/rate-plans/{ratePlanId}/rates-and-restrictions` | GET | Composed read for guest count, dates, availability, stop-sells, and rate totals |
| `blocks list` | `/blocks` | GET | Requires `dateType`, `startDate`, `endDate` |
| `inventory get` | `/room-types/{roomTypeId}/inventory` | GET | Requires `startDate`, `endDate` |
| `rates get` | `/rate-plans/{ratePlanId}/rates-and-restrictions` | GET | Requires `startDate`, `endDate` |
| `rates update` | `/rate-plans/{ratePlanId}/rates-and-restrictions` | PATCH | Requires `echoToken`, `restrictions[]` |
| `webhooks list` | `/webhooks` | GET | Webhook discovery |
| `webhooks create` | `/webhooks` | POST | Requires `url`, `hookType` |

## Reservation Query Rules

`dateType` is required:

- `checkIn` uses `startDate` and `endDate`.
- `checkOut` uses `startDate` and `endDate`.
- `updated` uses `startTime` and `endTime`.

## Rate Update Payload

```json
{
  "echoToken": "demo-rate-update-2026-07-04",
  "restrictions": [
    {
      "date": "2026-07-04",
      "rate": 385,
      "minStay": 2,
      "stopSell": false,
      "cta": true,
      "ctd": false
    }
  ]
}
```

The Resly docs note that the update endpoint may return HTTP 200 even when individual updates fail, so consumers must inspect `stats` and `result`.

## Sources

- https://docs.resly.com.au/llms.txt
- https://docs.resly.com.au/docs/authentication-1
- https://docs.resly.com.au/reference/retrieve-reservations
- https://docs.resly.com.au/reference/retrieve-rate-plan-rates-and-restrictions
- https://docs.resly.com.au/reference/update-rate-plan-rates-and-restrictions
- https://docs.resly.com.au/reference/create-a-webhook
