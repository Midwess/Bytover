//
//  People.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 6/3/25.
//

import SwiftUI
import Foundation

struct PeopleShareItem: View {
    var body: some View {
        HStack(spacing: 12) {
            ImageAsset.Owl.image
                .resizable()
                .scaledToFit()
                .frame(width: 35, height: 35)
                .padding(.all, 3)
                .background(Circle()
                    .fill(Color(ImageAsset.Owl.uiImage.backgroundColor)))
            VStack(alignment: .leading, spacing: 4) {
                Text("tiendvlp")
                    .foregroundColor(Theme.PrimaryText.color)
                    .modifier(Label1())
                
                Text("Send by email")
                    .modifier(Label2())
                    .padding(.trailing, 8)
                    .foregroundColor(Theme.PrimaryText.color.opacity(0.7))
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 5)
        .frame(maxWidth: .infinity)
        .clipShape(Capsule())
        .background(Capsule().fill(Theme.PrimaryText.color.opacity(0.1)))
        .overlay(Capsule().stroke(Theme.PrimaryText.color.opacity(0.1)))
    }
}

struct PeopleShareView: View {
    var body: some View {
        VStack(spacing: 16) {
            Button(action: {}) {
                HStack {
                    Text("Find or add new people here")
                        .modifier(Label1())
                        .foregroundStyle(Theme.PrimaryText.color)
                    Spacer()
                }
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity)
            .background(Theme.PrimaryText.color.opacity(0.15))
            .clipShape(Capsule())
            
            ScrollView(.horizontal) {
                HStack(spacing: 8) {
                    PeopleShareItem()
                    PeopleShareItem()
                    PeopleShareItem()
                }
            }
            .scrollIndicators(.hidden)
        }
    }
}

#Preview {
    PeopleShareView()
}
